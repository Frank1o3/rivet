package({
    name = "rivet",
    version = "0.1.6",

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

    cleanup = {
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

        local binary = src .. "/target/release/rivet-cli"

        if not ctx:exists(binary) then
            error("Rivet binary was not produced: " .. binary)
        end

        ctx:mkdir(dest .. "/usr/bin")

        ctx:copy(
            binary,
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

        if ctx:is_symlink(prefix .. "/bin/rivet") ~= true then
            ctx:symlink(
                dest .. "/usr/bin/rivet",
                prefix .. "/bin/rivet"
            )
        end
    end,

    uninstall = function(ctx)
        local prefix = ctx:prefix()

        if ctx:is_symlink(prefix .. "/bin/rivet") then
            ctx:remove_symlink(prefix .. "/bin/rivet")
        end
    end,
})
