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
            "cd '" .. src .. "' && ./autogen.sh && ./configure --prefix=/usr && make -j$(nproc)",
        })
    end,

    install = function(ctx)
        local src = ctx:source_dir()
        ctx:run("sh", { "-c", "cd '" .. src .. "' && make install" })
    end,
})
