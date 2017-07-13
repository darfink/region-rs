#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
/// This module exists solely to wrap the protection flags in a namespace, until
/// [associated constants](https://github.com/rust-lang/rust/issues/17841) are in stable.
pub mod Protection {
    bitflags! {
        /// Memory page protection constants.
        ///
        /// Determines the access rights for a specific page and/or region. Some
        /// combination of flags may not work depending on the OS (e.g macOS
        /// enforces pages to be readable).
        ///
        /// # Examples
        ///
        /// ```
        /// use region::Protection;
        ///
        /// let combine = Protection::Read | Protection::Write;
        /// let shorthand = Protection::ReadWrite;
        /// ```
        pub struct Flag: usize {
            /// No access allowed at all.
            const None             = 0;
            /// Read access; writing and/or executing data will panic.
            const Read             = (1 << 1);
            /// Write access; this flag alone may not work on all OS.
            const Write            = (1 << 2);
            /// Execute access; this may not be allowed depending on DEP.
            const Execute          = (1 << 3);
            /// Read and execute shorthand.
            const ReadExecute      = (Read.bits | Execute.bits);
            /// Read and write shorthand.
            const ReadWrite        = (Read.bits | Write.bits);
            /// Read, write and execute shorthand.
            const ReadWriteExecute = (Read.bits | Write.bits | Execute.bits);
            /// Write and execute shorthand.
            const WriteExecute     = (Write.bits | Execute.bits);
        }
    }
}
