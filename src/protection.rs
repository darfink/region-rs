#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

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
    pub struct Protection: usize {
        /// No access allowed at all.
        const None             = 0;
        /// Read access; writing and/or executing data will panic.
        const Read             = (1 << 1);
        /// Write access; this flag alone may not be supported on all OSs.
        const Write            = (1 << 2);
        /// Execute access; this may not be allowed depending on DEP.
        const Execute          = (1 << 3);
        /// Read and execute shorthand.
        const ReadExecute      = (Self::Read.bits | Self::Execute.bits);
        /// Read and write shorthand.
        const ReadWrite        = (Self::Read.bits | Self::Write.bits);
        /// Read, write and execute shorthand.
        const ReadWriteExecute = (Self::Read.bits | Self::Write.bits | Self::Execute.bits);
        /// Write and execute shorthand.
        const WriteExecute     = (Self::Write.bits | Self::Execute.bits);
    }
}
