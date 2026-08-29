package({
    name = "rivet",
    version = "0.1.4",

    description = "A general-purpose, cross-platform package manager",
    license = "BSD-3-Clause",
    homepage = "https://github.com/Frank1o3/rivet",

    source = {
        type = "git",
        url = "https://github.com/Frank1o3/rivet.git",
        branch = "main",
    },

    build_dependencies = {
        "rust",
    },

    build = function(ctx)
        ctx:run_in(ctx:source_dir(), "cargo", {
            "build",
            "--release",
            "--locked",
            "-p",
            "rivet-cli",
        })
    end,

    install = function(ctx)
        local src = ctx:source_dir()
        local dest = ctx:destdir()

        ctx:mkdir(dest .. "/usr/bin")

        ctx:copy(
            src .. "/target/release/rivet-cli",
            dest .. "/usr/bin/rivet"
        )

        ctx:chmod(
            dest .. "/usr/bin/rivet",
            "755"
        )
    end,

    post_install = function(ctx)
        local dest = ctx:destdir()
        local prefix = ctx:prefix()

        ctx:mkdir(prefix .. "/bin")

        ctx:symlink(
            dest .. "/usr/bin/rivet",
            prefix .. "/bin/rivet"
        )
    end,
})
