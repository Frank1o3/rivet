package({
    name = "cava",
    version = "1.0.0",

    description = "Cross-platform audio visualizer for the terminal",
    license = "MIT",
    homepage = "https://github.com/karlstav/cava",

    source = {
        type = "git",
        url = "https://github.com/karlstav/cava.git",
        tag = "1.0.0",
    },

    build = function(ctx)
        local src = ctx:source_dir()

        ctx:run("sh", {
            "-c",
            table.concat({
                "cd '" .. src .. "'",
                "./autogen.sh",
                "./configure --prefix=/usr",
                "make -j$(nproc)",
            }, " && "),
        })
    end,

    install = function(ctx)
        local src = ctx:source_dir()

        ctx:run("sh", {
            "-c",
            "cd '" .. src .. "' && make install",
        })
    end,

    post_install = function(ctx)
        local prefix = ctx:prefix()
        local dest = ctx:destdir()

        ctx:mkdir(prefix .. "/bin")

        if ctx:is_symlink(prefix .. "/bin/cava") ~= true then
            ctx:symlink(dest .. "/usr/bin/cava", prefix .. "/bin/cava")
        end
    end,

    uninstall = function(ctx)
        local prefix = ctx:prefix()

        if ctx:is_symlink(prefix .. "/bin/cava") then
            ctx:remove_symlink(prefix .. "/bin/cava")
        end
    end,
})
