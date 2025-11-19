fn main() {
    #[cfg(windows)]
    {
        // This will compile and link the manifest.rc file
        embed_resource::compile("manifest.rc", embed_resource::NONE);
    }
}
