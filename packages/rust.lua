package({
    name = "rust",
    version = "1.98.0",

    description = "Rust compiler and Cargo package manager",
    license = "MIT OR Apache-2.0",
    homepage = "https://www.rust-lang.org/",

    source = {
        type = "archive",
        url = "https://static.rust-lang.org/dist/2026-08-20/rust-1.98.0-x86_64-unknown-linux-gnu.tar.xz",
        sha256 = "ed8ee2df70909c88cbaf87a6cfa3920dac00b537de12a6abe6906641e0f5952f"
    },

    install = function(ctx)
        local src = ctx:source_dir()
        local dest = ctx:destdir()

        ctx:run("sh", {
            src .. "/install.sh",
            "--prefix=" .. dest .. "/usr",
            "--disable-ldconfig",
            "--without=rust-docs",
        })
    end,
})
