//! Driver for the 8259 Programmable Interrupt Controller.
//!
//! Only handles the usual case of two PICs in a cascading setup, where the
//! SLAVE is setup to cascade to the line 2 of the MASTER.

use crate::i386::pio::Pio;
use crate::io::Io;
use crate::sync::{Once, SpinLockIRQ};

bitflags! {
    /// The first control word sent to the PIC.
    struct ICW1: u8 {
        /// If this bit is set, ICW4 has to be read. If ICW4 is not needed, set
        /// ICW4 to 0
        const ICW4      = 0x01;
        /// Single. Means that this is the only 8259A in the system. If SINGLE
        // is 1, no ICW3 will be issued.
        const SINGLE    = 0x02;
        /// Call Address Interval. Used only in 8085, not 8086. 1=ISR's are 4
        /// bytes apart (0200, 0204, etc) 0=ISR's are 8 byte apart (0200, 0208,
        /// etc)
        const INTERVAL4 = 0x04;
        /// If LEVEL = 1, then the 8259A will operate in the level interrupt
        /// mode. Edge detect logic on the interrupt inputs will be disabled.
        const LEVEL     = 0x08;
        /// Should always be set to 1.
        const INIT      = 0x10;
    }
}

/// ICW4: 8086/88 (MCS-80/85) mode.
const ICW4_8086: u8     = 0x01;       /* 8086/88 (MCS-80/85) mode */
//const icw4_auto         = 0x02;       /* Auto (normal) EOI */
//const icw4_buf_slave    = 0x08;       /* Buffered mode/slave */
//const icw4_buf_master   = 0x0C;       /* Buffered mode/master */
//const icw4_sfnm         = 0x10;       /* Special fully nested (not) */

/// The PIC manager.
static PIC: Once<Pic> = Once::new();

/// Acquires a reference to the PIC, initializing it if it wasn't already setup.
pub fn get() -> &'static Pic {
    PIC.call_once(|| unsafe {
        Pic::new()
    })
}

/// Initializes the PIC if it has not yet been initialized. Otherwise, does nothing.
pub fn init() {
    PIC.call_once(|| unsafe {
        Pic::new()
    });
}

/// A single PIC8259 device.
#[derive(Debug)]
struct InternalPic {
    /// The PIC's COMMAND IO port.
    port_cmd: Pio<u8>,
    /// The PIC's DATA IO port.
    port_data: Pio<u8>
}

/// A master/slave PIC setup, as commonly found on IBM PCs.
#[derive(Debug)]
pub struct Pic {
    /// The master PIC.
    master: SpinLockIRQ<InternalPic>,
    /// The slave PIC, cascaded on line 2 of `.master`
    slave: SpinLockIRQ<InternalPic>,
}

impl Pic {
    /// Creates a new PIC, and initializes it.
    ///
    /// Interrupts will be mapped to IRQ [32..48]
    ///
    /// # Safety
    ///
    /// This should only be called once! If called more than once, then both Pics instances
    /// will share the same underlying Pios, but different mutexes protecting them!
    unsafe fn new() -> Pic {
        Pic {
            master: SpinLockIRQ::new(InternalPic::new(0x20, true, 32)),
            slave: SpinLockIRQ::new(InternalPic::new(0xA0, false, 32 + 8)),
        }
    }

    /// Mask the given IRQ number. Will redirect the call to the right Pic device.
    pub fn mask(&self, irq: u8) {
        if irq < 8 {
            self.master.lock().mask(irq);
        } else {
            self.slave.lock().mask(irq - 8);
        }
    }

    /// Unmask the given IRQ number. Will redirect the call to the right Pic device.
    pub fn unmask(&self, irq: u8) {
        if irq < 8 {
            self.master.lock().unmask(irq);
        } else {
            self.slave.lock().unmask(irq - 8);
        }
    }

    /// Reads the PIC interrupt mask. Used for debug purposes.
    ///
    /// LSB is irq 0, MSB is irq 15.
    pub fn get_mask(&self) -> u16 {
        u16::from(self.master.lock().get_mask()) | (u16::from(self.slave.lock().get_mask()) << 8)
    }

    /// Acknowledges an IRQ, allowing the PIC to send a new IRQ on the next
    /// cycle.
    pub fn acknowledge(&self, irq: u8) {
        self.master.lock().acknowledge();
        if irq >= 8 {
            self.slave.lock().acknowledge();
        }
    }
}

impl InternalPic {
    /// Setup the 8259 pic. Redirect the IRQ to the chosen interrupt vector.
    ///
    /// # Safety
    ///
    /// The port should map to a proper PIC device. Sending invalid data to a
    /// random device can lead to memory unsafety. Furthermore, care should be
    /// taken not to share the underlying Pio.
    unsafe fn new(port_base: u16, is_master: bool, vector_offset: u8) -> InternalPic {
        let mut pic = InternalPic {
            port_cmd: Pio::new(port_base),
            port_data: Pio::new(port_base + 1)
        };

        // save masks
        let mask_backup = pic.port_data.read();

        // starts the initialization sequence (in cascade mode)
        pic.port_cmd.write((ICW1::INIT | ICW1::ICW4).bits());

        // ICW2: Master PIC vector offset
        pic.port_data.write(vector_offset);

        // ICW3: tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
        pic.port_data.write(if is_master { 4 } else { 2 });

        // ICW4: ??
        pic.port_data.write(ICW4_8086);

        // restore saved masks.
        pic.port_data.write(mask_backup);

        pic
    }

    /// Acknowledges an IRQ, allowing the PIC to send a new IRQ on the next
    /// cycle.
    pub fn acknowledge(&mut self) {
        unsafe {
            self.port_cmd.write(0x20);
        }
    }

    /// Mask the given IRQ
    pub fn mask(&mut self, irq: u8) {
        self.port_data.writef(1 << irq, true);
    }

    /// Unmask the given IRQ
    pub fn unmask(&mut self, irq: u8) {
        self.port_data.writef(1 << irq, false);
    }

    /// Read the IRQ mask. Used mostly for debug purposes.
    pub fn get_mask(&self) -> u8 {
        self.port_data.read()
    }
}
