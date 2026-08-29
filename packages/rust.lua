package({
    name = "rust",
    version = "1.98.0",

    description = "Rust compiler and Cargo package manager",
    license = "MIT OR Apache-2.0",
    homepage = "https://www.rust-lang.org/",

    provides_check = {
        command = "rustc",
        version_flag = "--version"
    },

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

    post_install = function(ctx)
        local prefix = ctx:prefix()
        local dest = ctx:destdir()
        local bin_dir = dest .. "/usr/bin"

        ctx:mkdir(prefix .. "/bin")

        for _, name in ipairs({ "rustc", "cargo", "rustdoc", "rust-gdb", "rust-lldb" }) do
            local target = bin_dir .. "/" .. name
            if ctx:exists(target) and ctx:is_symlink(prefix .. "/bin/" .. name) ~= true then
                ctx:symlink(target, prefix .. "/bin/" .. name)
            end
        end
    end,

    uninstall = function(ctx)
        local prefix = ctx:prefix()

        for _, name in ipairs({ "rustc", "cargo", "rustdoc", "rust-gdb", "rust-lldb" }) do
            local link = prefix .. "/bin/" .. name
            if ctx:is_symlink(link) then
                ctx:remove_symlink(link)
            end
        end
    end,
})
