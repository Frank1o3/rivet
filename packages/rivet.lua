package({
    name = "rivet",
    version = "0.1.8",

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

        local bin = src .. "/target/release/rivet-cli"
        local dep = src .. "/target/release/rivet-cli.d"

        if not ctx:exists(bin) then
            error("Rivet binary was not produced: " .. bin)
        end

        ctx:mkdir(dest .. "/usr/bin")

        ctx:copy(
            bin,
            dest .. "/usr/bin/rivet"
        )

        ctx:chmod(
            dest .. "/usr/bin/rivet",
            "755"
        )

        if ctx:exists(dep) then
            ctx:copy(
                dep,
                dest .. "/usr/bin/rivet.d"
            )
        end
    end,

    post_install = function(ctx)
        local dest = ctx:destdir()
        local prefix = ctx:prefix()

        ctx:mkdir(prefix .. "/bin")

        local link = prefix .. "/bin/rivet"

        if ctx:is_symlink(link) ~= true then
            ctx:symlink(
                dest .. "/usr/bin/rivet",
                link
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
