use std::env;
use std::path::PathBuf;

static FILES: &[&str] = &[
    #[cfg(feature = "stb_easy_font")]
    "src/stb_easy_font.c",
    #[cfg(feature = "stb_dxt")]
    "src/stb_dxt.c",
    #[cfg(feature = "stb_image")]
    "src/stb_image.c",
    #[cfg(feature = "stb_image_write")]
    "src/stb_image_write.c",
    #[cfg(feature = "stb_rect_pack")]
    "src/stb_rect_pack.c",
    #[cfg(feature = "stb_image_resize")]
    "src/stb_image_resize.c",
    #[cfg(feature = "stb_truetype")]
    "src/stb_truetype.c",
];

use std::borrow::Borrow;
use std::fmt::{self};
use std::fmt::{Display, Formatter};


#[derive(Clone, Debug)]
pub struct Target {
    pub architecture: String,
    pub vendor: String,
    pub system: String,
    pub abi: Option<String>,
}

impl Target {
    pub fn as_strs(&self) -> (&str, &str, &str, Option<&str>) {
        (
            self.architecture.as_str(),
            self.vendor.as_str(),
            self.system.as_str(),
            self.abi.as_deref(),
        )
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            &self.architecture, &self.vendor, &self.system
        )?;

        if let Some(ref abi) = self.abi {
            write!(f, "-{}", abi)
        } else {
            Result::Ok(())
        }
    }
}

pub fn ndk() -> String {
    std::env::var("ANDROID_NDK").expect("ANDROID_NDK variable not set")
}

pub fn target_arch(arch: &str) -> &str {
    match arch {
        "armv7" => "arm",
        "aarch64" => "arm64",
        "i686" => "x86",
        arch => arch,
    }
}

fn main() {
    let target_str = env::var("TARGET").unwrap();
    let target: Vec<String> = target_str.split('-').map(|s| s.into()).collect();
    if target.len() < 3 {
        assert!(!(target.len() < 3), "Failed to parse TARGET {}", target_str);
    }

    let abi = if target.len() > 3 {
        Some(target[3].clone())
    } else {
        None
    };

    let target = Target {
        architecture: target[0].clone(),
        vendor: target[1].clone(),
        system: target[2].clone(),
        abi,
    };
    println!("cargo:rerun-if-changed=build.rs");


    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_dir.join("bindings.rs");

    if FILES.is_empty() {
        // Write an empty file so `include!` won't fail the build
        std::fs::write(bindings_path, "").unwrap();
        return;
    }

    let mut builder = bindgen::builder();
    for f in FILES {
        builder = builder.header(*f)
    }

    match target.system.borrow() {
        "android" | "androideabi" => {
            builder = builder.clang_arg(&format!("--sysroot={}/sysroot", ndk()));
        }
        "ios" | "darwin" => {

            let system = target.system.as_str();
            let env_target = env::var("TARGET").unwrap();
            let directory = sdk_path(&env_target).ok();
            builder = add_bindgen_root(
                directory.as_ref().map(String::as_ref),
                &env_target,
                builder,
            );

            println!("system {:?} x abi : {:?} x arch: {:?}",system, target.abi.as_ref() , target.architecture.as_str());

            if system == "ios" {
                builder = builder.clang_arg("-miphoneos-version-min=10.0");


                if target.abi.as_deref() == Some("sim") && target.architecture.as_str() == "aarch64" {
                    builder = builder.clang_arg("-mios-simulator-version-min=14.0");
                }

            } else {
                builder = builder.clang_arg("-miphoneos-version-min=14.0");
            }
        }
        _ => {}
    }

    builder
        .allowlist_function("stb.*")
        .allowlist_type("stb.*")
        .allowlist_var("stb.*")
        .clang_arg("-miphoneos-version-min=10.0")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file(bindings_path)
        .expect("Failed to write bindings file");

    let mut builder = cc::Build::new();
    builder.flag_if_supported("-Wno-implicit-function-declaration");

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

    match target.system.borrow() {
        "android" | "androideabi" => {
            builder.flag(&format!("--sysroot={}/sysroot", ndk()));
        }
        "ios" | "darwin" => {
            let target = env::var("TARGET").unwrap();
            let directory = sdk_path(&target).ok();
            add_cc_root(
                directory.as_ref().map(String::as_ref),
                &target,
                &mut builder,
            );
        }
        _ => {}
    }

    builder.files(FILES).warnings(false).compile("libstb");
}

fn sdk_path(target: &str) -> Result<String, std::io::Error> {
    use std::process::Command;
    let sdk = if target.contains("apple-darwin")
        || target == "aarch64-apple-ios-macabi"
        || target == "x86_64-apple-ios-macabi"
    {
        "macosx"
    } else if target == "x86_64-apple-ios"
        || target == "i386-apple-ios"
        || target == "aarch64-apple-ios-sim"
    {
        "iphonesimulator"
    } else if target == "aarch64-apple-ios"
        || target == "armv7-apple-ios"
        || target == "armv7s-apple-ios"
    {
        "iphoneos"
    } else {
        unreachable!();
    };

    let output = Command::new("xcrun")
        .args(&["--sdk", sdk, "--show-sdk-path"])
        .output()?
        .stdout;
    let prefix_str = std::str::from_utf8(&output).expect("invalid output from `xcrun`");
    Ok(prefix_str.trim_end().to_string())
}

fn add_bindgen_root(
    sdk_path: Option<&str>,
    target: &str,
    mut builder: bindgen::Builder,
) -> bindgen::Builder {
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");

    println!("target {:?}", target);

    // let build_sdk_target = if target == "aarch64-apple-ios" {
    //     "-miphoneos-version-min=9.0"
    // } else if target == "aarch64-apple-ios-sim" {
    //     "-mios-simulator-version-min=14.0"
    // } else {
    //     "-mios-simulator-version-min=9.0"
    // };

    // builder = builder.clang_arg(build_sdk_target);
    


    let target = if target == "aarch64-apple-ios" || target == "x86_64-apple-ios" {
        target.to_string()
    } else if target == "aarch64-apple-ios-sim" {
        "arm64-apple-ios14.0.0-simulator".to_string()
    } else {
        // todo support other apple targets;
        panic!("Target not supported");
    };

   
    builder = builder.clang_arg(format!("--target={}", target));

    if let Some(sdk_path) = sdk_path {
        builder = builder.clang_args(&["-isysroot", sdk_path]);
    }

    return builder;
}

fn add_cc_root(sdk_path: Option<&str>, target: &str, builder: &mut cc::Build) {
    println!("cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS");

    // let build_sdk_target = if target == "aarch64-apple-ios" {
    //     "-miphoneos-version-min=9.0"
    // } else if target == "aarch64-apple-ios-sim" {
    //     "-mios-simulator-version-min=14.0"
    // } else {
    //     "-mios-simulator-version-min=9.0"
    // };

    // builder.flag(build_sdk_target);

    let target = if target == "aarch64-apple-ios" || target == "x86_64-apple-ios" {
        target.to_string()
    } else if target == "aarch64-apple-ios-sim" {
        builder.flag("-m64");
        "arm64-apple-ios14.0.0-simulator".to_string()
    }else {
        // todo support other apple targets;
        panic!("Target not supported");
    };

    if target == "x86_64-apple-ios" {
        builder.flag("-mios-simulator-version-min=10.0");
    }else if target == "aarch64-apple-ios" {
        builder.flag("-miphoneos-version-min=10.0");
    }
   
    builder.flag(&format!("--target={}", target));

    if let Some(sdk_path) = sdk_path {
        builder.flag(&format!("-isysroot{}", sdk_path));
    }
}
