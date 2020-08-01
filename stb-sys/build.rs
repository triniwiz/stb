use std::env;
use std::path::PathBuf;

static FILES: &[&str] = &[
    #[cfg(feature = "stb_easy_font")]
    "src/stb_easy_font.c",
    #[cfg(feature = "stb_dxt")]
    "src/stb_dxt.c",
    #[cfg(feature = "stb_image")]
    "src/stb_image.c",
];

fn main() {
    if FILES.is_empty() {
        // Nothing to do
        return;
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut builder = bindgen::builder();
    for f in FILES {
        builder = builder.header(*f)
    }
    builder
        .whitelist_function("stb.*")
        .whitelist_type("stb.*")
        .whitelist_var("stb.*")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings file");

    let mut builder = cc::Build::new();

    #[cfg(feature = "stb_dxt")]
    {
        #[cfg(feature = "stb_dxt_use_rounding_bias")]
        builder.define("STB_DXT_USE_ROUNDING_BIAS", "1");
    }

    #[cfg(feature = "stb_image")]
    {
        #[cfg(feature = "stbi_no_linear")]
        builder.define("STBI_NO_LINEAR", "1");

        #[cfg(feature = "stbi_no_jpeg")]
        builder.define("STBI_NO_JPEG", "1");

        #[cfg(feature = "stbi_no_png")]
        builder.define("STBI_NO_PNG", "1");

        #[cfg(feature = "stbi_no_bmp")]
        builder.define("STBI_NO_BMP", "1");

        #[cfg(feature = "stbi_no_psd")]
        builder.define("STBI_NO_PSD", "1");

        #[cfg(feature = "stbi_no_gif")]
        builder.define("STBI_NO_GIF", "1");

        #[cfg(feature = "stbi_no_hdr")]
        builder.define("STBI_NO_HDR", "1");

        #[cfg(feature = "stbi_no_pic")]
        builder.define("STBI_NO_PIC", "1");

        #[cfg(feature = "stbi_no_pnm")]
        builder.define("STBI_NO_PNM", "1");
    }

    builder.files(FILES).warnings(false).compile("libstb");
}
