error_chain! {
    foreign_links {
        ProcfsIo(::std::io::Error);
    }

    errors {
        /// The supplied address is null.
        Null { display("address must not be null") }
        /// The queried memory is free.
        Free { display("address does not contain allocated memory") }
        /// Invalid regex group count.
        ProcfsMatches { display("invalid captue group count") }
        /// A system call failed.
        SystemCall(error: ::errno::Errno) {
            description("system call failed")
            display("system call failed with: {}", error)
        }
        /// macOS kernal call failed
        MachRegion(code: ::libc::c_int) {
            description("macOS kernel call failed")
            display("kernel call failed with: {}", code)
        }
    }
}
