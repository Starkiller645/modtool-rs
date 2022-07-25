#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::*;
use std::fs::File;
use std::path::Path;
use std::io::{Cursor, Write};
use std::io::prelude::*;
use chrono::{DateTime, Utc, SecondsFormat};
use std::collections::HashMap;
use std::process;
use std::rc::Rc;
use std::env;
use serde::{Serialize, Deserialize};
use std::cmp::Ordering;
use async_std::stream::StreamExt;
use lazy_static::*;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static APP_VERSION: &'static str = "2.1.0";

#[derive(Copy, Clone)]
enum Page {
    ManifestDownloadPage,
    HomePage,
    ProfilePage,
    DownloadPage,
    Complete,
    FabricCheckPage,
    ForgeCheckPage,
    JavaCheckPage
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq)]
enum ModLoader {
    Fabric,
    Forge
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
struct ForgeVersion {
    minecraft: String,
    forge: String
}

#[derive(Clone)]
struct AppState {
    page: Page,
    selected_profile: i32,
    manifest: Manifest,
    download_list: ModDownloads
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum ModProvider {
    CurseForge,
    Modrinth,
    Creator,
    Unknown
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Props)]
struct Mod {
    name: String,
    url: String,
    version: String,
    provider: ModProvider,
    size: i32
}

#[derive(Clone, PartialEq)]
struct ModDownload {
    name: String,
    url: String,
    version: String,
    bytes_total: i32,
    bytes_read: i32,
    status: Download,
    provider: ModProvider
}

#[derive(Serialize, Deserialize, Clone)]
struct Profile {
    meta: ProfileMeta,
    mods: Vec<Mod>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
struct ProfileMeta {
    name: String,
    loader: ModLoader,
    version: String,
    id: i32
}

#[derive(Clone, PartialEq)]
enum Download {
    InProgress,
    Complete
}


#[derive(Clone)]
struct ModDownloads {
    downloads: Vec<ModDownload>
}

#[derive(Serialize, Deserialize, Clone)]
struct Manifest {
    profiles: Vec<Profile>
}

#[derive(Serialize, Deserialize, Clone)]
struct LauncherProfile {
    created: Option<String>,
    icon: String,    // Should be base64 encoded PNG
    javaArgs: Option<String>,
    lastUsed: String,
    lastVersionId: String,
    name: String,
    r#type: String
}

#[derive(Serialize, Deserialize, Clone)]
struct LauncherSettings {
    crashAssistance: bool,
    enableAdvanced: bool,
    enableAnalytics: bool,
    enableHistorical: bool,
    enableReleases: bool,
    enableSnapshots: bool,
    keepLauncherOpen: bool,
    profileSorting: String,
    showGameLog: bool,
    showMenu: bool,
    soundOn: bool
}

#[derive(Serialize, Deserialize, Clone)]
struct LauncherProfiles {
    profiles: HashMap<String, LauncherProfile>,
    settings: LauncherSettings,
    version: i32
}

struct MCData {
    base_dir: String,
    mods_dir: String,
//    packs_dir: String,
    profiles_dir: String
}

impl Manifest {
    fn lookup(&self, id: i32) -> Profile {
        for profile in self.profiles.clone() {
            if profile.meta.id == id {
                return profile.clone();
            }
        }
        return self.profiles[0].clone();
    }
}

#[derive(Clone)]
struct ManifestString(String);

static STATE: Atom<AppState> = |_| AppState {
    page: Page::ManifestDownloadPage,
    selected_profile: 0,
    manifest: Manifest {
        profiles: Vec::new()
    },
    download_list: ModDownloads {
        downloads: Vec::new()
    }
};


static NUM_DOWNLOADS: Atom<i32> = |_| {
    0
};

/*
struct DownloadHandler {
    current_downloads: i32,
    max_downloads: i32
}

impl DownloadHandler {
    async fn download() {}
}
*/

lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
    static ref CACHE_DIR: String =  {
        match cfg!(windows) {
            true => env::var("APPDATA").unwrap() + "\\modtool-rs\\cache\\",
            false => env::var("HOME").unwrap() + "/.cache/modtool-rs/"
        }
    };
    static ref CONFIG_DIR: String = {
        match cfg!(windows) {
            true => env::var("APPDATA").unwrap() + "\\modtool-rs",
            false => env::var("HOME").unwrap() + "/.config/modtool-rs"
        }
    };
    static ref MC_DATA: MCData = { 
        let base_dir = match cfg!(windows) {
            true => { env::var("APPDATA").unwrap() + "\\.minecraft\\"},
            false => { env::var("HOME").unwrap() + "/.minecraft/" }
        };
        MCData {
            base_dir: base_dir.to_string(),
            mods_dir: { base_dir.clone() + "mods" }.to_string(),
            profiles_dir: { base_dir.clone() + "versions" }.to_string(),
//            packs_dir: { base_dir.clone() + "resourcepacks" }.to_string()
        }
    };
}

fn init_dirs() {
    std::fs::create_dir_all(CONFIG_DIR.as_str()).unwrap();
    std::fs::create_dir_all(CACHE_DIR.as_str()).unwrap();
}

fn main() {
    init_dirs();
    dioxus::desktop::launch_cfg(App, |c| {
        c.with_custom_head("<link href=\"https://tallie.dev/modtool/assets/tailwind.css\" rel=\"stylesheet\" /><style>html {background: #334155; display: flex; flex-direction: column;}</style><script src=\"https://kit.fontawesome.com/a  0e919fade.js\" crossorigin=\"anonymous\"></script><link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/@fortawesome/fontawesome-free@6.1.1/css/fontawesome.min.css\" integrity=\"sha384-zIaWifL2YFF1qaDiAo0JFgsmasocJ/rqu7LKYH8CoB  EXqGbb9eO+Xi3s6fQhgFWM\" crossorigin=\"anonymous\">".to_string())
    });
}

fn App(cx: Scope) -> Element {

    cx.render(rsx! {
        PageRouter {}
    })
}

fn PageRouter(cx: Scope) -> Element {
    let state = use_read(&cx, STATE);

    cx.render(rsx! {
        div {
            class: "bg-slate-800 rounded-xl inset-5 absolute flex p-6 flex-col",
            match state.page {
                Page::ManifestDownloadPage => {
                    rsx! { ManifestPage {} }
                },
                Page::HomePage => {
                    rsx! { HomePage {} }
                },
                Page::ProfilePage => {
                    rsx! { ProfilePage {} }
                },
                Page::DownloadPage => {
                    rsx! { DownloadPage {} }
                },
                Page::Complete => {
                    rsx! { FinishedPage {} }
                }
                Page::JavaCheckPage => {
                    rsx! { JavaCheckPage {} }
                },
                Page::ForgeCheckPage => {
                    rsx! { ForgeCheckPage {} }
                },
                Page::FabricCheckPage => {
                    rsx! { FabricCheckPage {} }
                },
            }
        }
    })
}

fn HomePage(cx: Scope) -> Element {

    let atoms = use_atom_root(&cx);
    let state = use_read(&cx, STATE);

    //use_coroutine(&cx, |rx| to_manifest_page(rx, atoms.clone()));

    cx.render(rsx! {
        div {
            id: "homepage",
            class: "flex-1 flex-col flex justify-center",
            style: "{{font-family: \"Bebas Neue\", sans-serif;}}",
            p {
              class: "text-6xl text-slate-100 mx-auto text-center",
              "ModTool ",
              span {
                  class: "text-orange-600 font-bold",
                  "RS"
              },
              span {
                  class: "text-2xl text-slate-600",
                  " v{APP_VERSION}"
              }
            }
            button {
              onclick: move |_| {
                  match state.page {
                      Page::HomePage => {
                          let mut state_cpy = state.clone();
                          state_cpy.page = Page::JavaCheckPage;
                          atoms.set(STATE.unique_id(), state_cpy);
                      }
                      _ => {}
                  }
              },
              class: "hover:bg-green-700 bg-green-500 rounded-xl p-6 m-6 mx-auto",
              img {
                  src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                  height: "32",
                  width: "32",
                  class: "mx-auto fill-slate-100"
              },
            }
        }
    })
}

fn FinishedPage(cx: Scope) -> Element {
    let state = use_read(&cx, STATE);

	let atoms = use_atom_root(&cx);

    //use_coroutine(&cx, |rx| to_manifest_page(rx, atoms.clone()));
    let download_number = state.download_list.downloads.len().clone();
    let profile_name = state.manifest.lookup(state.selected_profile).meta.name.clone();

    cx.render(rsx! {
        div {
            id: "completepage",
            class: "flex-1 flex-col flex justify-center",
            style: "{{font-family: \"Bebas Neue\", sans-serif;}}",
            h2 {
				class: "text-6xl text-slate-100 mx-auto text-center font-bold",
				"Completed!"
            },
            div {
                class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto",
                p {
                    class: "text-xl text-slate-300 mx-auto text-center",
                    "Using profile ",
                    span {
                        class: "text-orange-600 font-bold",
                        "{profile_name}"
                    },
                    ":"
                },
                p {
                    class: "text-xl text-slate-300 mx-auto text-center font-bold",
                    "Downloaded and installed ",
                    span {
                        class: "text-cyan-300",
                        "{download_number} "
                    },
                    "mods."
                },
                p {
                    class: "text-sm text-slate-500 italic text-center",
                    "You can safely close this application now."
                }
            },
			button {
				class: "bg-green-500 hover:bg-green-700 rounded-xl justify-self-end p-6 self-center",
				onclick: move |_| {
					let mut state_cpy = state.clone();
					state_cpy.page = Page::ProfilePage;
					atoms.set(STATE.unique_id(), state_cpy);
				},
				img {
					src: "https://tallie.dev/modtool/assets/fa-home.svg",
					height: "32",
					width: "32",
					class: "mx-auto fill-slate-100"
				}
			}
        }
    })
}

async fn java_check(has_java: UseState<bool>, check_complete: UseState<bool>, java_version: UseState<String>) {
    let com = "java";
    let args = match cfg!(windows) {
        true => &["-version"],
        false => &["--version"]
    };
    let process = match cfg!(windows) {
        true => {
            match process::Command::new(com)
            .args(args)
            .output() {
                Ok(process) => process,
                Err(_) => {
                    check_complete.set(true);
                    has_java.set(false);
                    return
                }
            }
        },
        false => {
            match process::Command::new(com)
            .args(args)
            .output() {
                Ok(process) => process,
                Err(_) => {
                    check_complete.set(true);
                    has_java.set(false);
                    return
                }
            }
        }

    };
    let text = String::from_utf8(process.stdout).unwrap();
    let text_lines: Vec<&str> = text.split("\n").collect();
    java_version.set(format!("{}", text_lines[0]));
    check_complete.set(true);
    has_java.set(true);
}

async fn forge_install(mc_version: String, found_forge: UseState<bool>, check_complete: UseState<bool>, forge_ver: UseState<String>) {
    let manifest_res = HTTP_CLIENT
        .get("https://tallie.dev/modtool/forge_versions.json")
        .send()
        .await.unwrap()
        .text()
        .await.unwrap();
    let forge_manifest: Vec<ForgeVersion> = serde_json::from_str(manifest_res.clone().as_str()).unwrap();
    let mut forge_version: String = String::from("");
    for version in forge_manifest {
        if version.minecraft == mc_version {
            forge_version = version.forge;
        }
    }

    let profiles_dir = MC_DATA.profiles_dir.clone();
    let current_installs = std::fs::read_dir(profiles_dir).unwrap();
    let mut found = false;
    for version in current_installs {
        if format!("{}-forge-{}", mc_version, forge_version) == version.unwrap().file_name().into_string().unwrap() {
            found = true;
        }
    }
    if found {
        for file in std::fs::read_dir(MC_DATA.mods_dir.clone()).unwrap() {
            let file = file.unwrap();
            match std::fs::remove_file(file.path()) {
                Ok(_) => {},
                Err(_) => {}
            }
        }
        found_forge.set(true);
        check_complete.set(true);
        forge_ver.set(format!("{}-forge-{}", mc_version, forge_version));
        return
    }

    let installer_url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{0}-{1}/forge-{0}-{1}-installer.jar", mc_version, forge_version);
    let res = HTTP_CLIENT
        .get(installer_url.clone())
        .header("User-Agent", format!("Starkiller645/modtool_rs/{APP_VERSION} (tallie@tallie.dev)"))
        .send()
        .await.unwrap();

    let url = format!("{}", installer_url.clone());
    let path = Path::new(&url);
    let filename = path.file_name().unwrap();
    let filepath = format!("{}{}", CACHE_DIR.as_str(), filename.to_str().unwrap());
    let mut fhandle = File::create(filepath.clone()).unwrap();
    let mut content = Cursor::new(res.bytes().await.unwrap());
    std::io::copy(&mut content, &mut fhandle).unwrap();

    let com = "java";
    let args = &["-jar", filepath.as_str()];
    #[cfg(target_os = "windows")]
    {
        let _com = process::Command::new(com)
            .args(args)
            .creation_flags(0x08000000)
            .output().unwrap();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _com = process::Command::new(com)
            .args(args)
            .output().unwrap();
    }    
    let profiles_dir = MC_DATA.profiles_dir.clone();
    let current_installs = std::fs::read_dir(profiles_dir).unwrap();
    let mut found = false;
    for version in current_installs {
        if version.unwrap().file_name().into_string().unwrap().contains(format!("{}-forge", mc_version).as_str()) {
            found = true;
        }
    }


    if found {
        for file in std::fs::read_dir(MC_DATA.mods_dir.clone()).unwrap() {
            let file = file.unwrap();
            match std::fs::remove_file(file.path()) {
                Ok(_) => {},
                Err(_) => {}
            }
        }
        found_forge.set(true);
        check_complete.set(true);
        forge_ver.set(format!("{}-forge-{}", mc_version, forge_version));
        found_forge.needs_update();
        check_complete.needs_update();
    } else {
        found_forge.set(false);
        check_complete.set(true);
        found_forge.needs_update();
        check_complete.needs_update();
    }
}

async fn fabric_install(mc_version: String, found_fabric: UseState<bool>, check_complete: UseState<bool>, fabric_version: UseState<String>) {
   let profiles_dir = MC_DATA.profiles_dir.clone();
    let current_installs = std::fs::read_dir(profiles_dir).unwrap();
    let mut found = false;
    let mut ver = String::from("");
    for version in current_installs {
        let fname = version.unwrap().file_name().into_string().unwrap();
        if fname.contains("fabric-loader") && fname.contains(mc_version.as_str()) {
            found = true;
            let info_arr = fname.split("-").collect::<Vec<&str>>();
            if info_arr.len() > 2 {
                ver = String::from(info_arr[2]);
            } else {
                ver = String::from("unknown");
            }
        }
    }
    if found {
        for file in std::fs::read_dir(MC_DATA.mods_dir.clone()).unwrap() {
            let file = file.unwrap();
            match std::fs::remove_file(file.path()) {
                Ok(_) => {},
                Err(_) => {}
            }
        }
        found_fabric.set(true);
        check_complete.set(true);
        fabric_version.set(format!("Fabric {} for Minecraft {}", ver, mc_version));
        return
    }

    let installer_url = format!("https://maven.fabricmc.net/net/fabricmc/fabric-installer/0.11.0/fabric-installer-0.11.0.jar");
    let res = HTTP_CLIENT
        .get(installer_url.clone())
        .header("User-Agent", format!("Starkiller645/modtool_rs/{APP_VERSION} (tallie@tallie.dev)"))
        .send()
        .await.unwrap();

    let url = format!("{}", installer_url.clone());
    let path = Path::new(&url);
    let filename = path.file_name().unwrap();
    let filepath = format!("{}{}", CACHE_DIR.as_str(), filename.to_str().unwrap());
    let mut fhandle = File::create(filepath.clone()).unwrap();
    let mut content = Cursor::new(res.bytes().await.unwrap());
    std::io::copy(&mut content, &mut fhandle).unwrap();

    let com = "java";
    let args = &["-jar", filepath.as_str(), "client", "-mcversion", mc_version.as_str(), "-dir", MC_DATA.base_dir.as_str()];

    #[cfg(target_os = "windows")]
    {
        let _com = process::Command::new(com)
            .args(args)
            .creation_flags(0x08000000)
            .output().unwrap();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _com = process::Command::new(com)
            .args(args)
            .output().unwrap();
    }

    let profiles_dir = MC_DATA.profiles_dir.clone();
    let current_installs = std::fs::read_dir(profiles_dir).unwrap();
    let mut found = false;
    for version in current_installs {
        let fname = version.unwrap().file_name().into_string().unwrap();
        if fname.contains("fabric-loader") && fname.contains(mc_version.as_str()) {
            found = true;
        }
    }


    if found {
        for file in std::fs::read_dir(MC_DATA.mods_dir.clone()).unwrap() {
            let file = file.unwrap();
            match std::fs::remove_file(file.path()) {
                Ok(_) => {},
                Err(_) => {}
            }
        }
        found_fabric.set(true);
        check_complete.set(true);
        fabric_version.set(format!("Fabric {} for Minecraft {}", ver, mc_version));
        found_fabric.needs_update();
        check_complete.needs_update();
    } else {
        found_fabric.set(false);
        check_complete.set(true);
        found_fabric.needs_update();
        check_complete.needs_update();
    }
}

fn FabricCheckPage(cx: Scope) -> Element {
    let check_complete = use_state(&cx, || false);
    let has_fabric = use_state(&cx, || false);
    let fabric_version = use_state(&cx, || String::from(""));

    let atoms = use_atom_root(&cx);
    let state = use_read(&cx, STATE);

    let mc_version = state.manifest.lookup(state.selected_profile).clone().meta.version;

    use_future(&cx, (), |_| fabric_install(mc_version, has_fabric.clone(), check_complete.clone(), fabric_version.clone()));

    cx.render(rsx! {
        div {
            id: "fabriccheckpage",
            class: "flex-1 flex-col flex justify-center w-full",
            match *check_complete.current() {
                true => rsx! {
                    div {
                        class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto flex-col w-1/2",
                        match *has_fabric.current() {
                            true => {
                                let mut state_cpy = state.clone();
                                state_cpy.page = Page::DownloadPage;
                                atoms.set(STATE.unique_id(), state_cpy);
                                rsx! {""}
                            },
                            false => {
                                rsx! {
                                    div {
                                        class: "flex flex-col",
                                        p {
                                            class: "text-xl text-orange-600 font-bold text-center",
                                            "Could not install Fabric!"
                                        },
                                        p {
                                            class: "text-xl text-slate-100 text-center",
                                            "Try re-running the program, or download and install Fabric manually."
                                        },
                                        p {
                                            class: "text-sm text-slate-500 italic text-center",
                                            "Fabric Mod Loader can be downloaded from ",
                                            a {
                                                class: "text-sm text-sky-500 italic underline",
                                                href: "https://fabricmc.net/use/installer/",
                                                "https://fabricmc.net/use/installer/"
                                            }
                                        },
                                        button {
                                            class: "bg-red-500 hover:bg-red-700 rounded-xl p-6 m-6 mb-0 mx-auto align-center",
                                            onclick: move |_| {
                                                let mut state_cpy = state.clone();
                                                state_cpy.page = Page::ProfilePage;
                                                atoms.set(STATE.unique_id(), state_cpy);
                                            },
                                            img {
                                                src: "https://tallie.dev/modtool/assets/fa-arrow-left.svg",
                                                height: "32",
                                                width: "32",
                                                class: "mx-auto fill-slate-100"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                false => {
                    rsx! {
                        h2 {
                            class: "text-6xl text-slate-100 mx-auto text-center font-bold p-6",
                            "Installing Fabric..."
                        },
                        div {
                            class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto flex-col w-1/2",
                            p {
                                class: "text-xl text-slate-100 font-bold text-center",
                                "Fabric is now installing."
                            },
                            p {
                                class: "text-sm text-slate-500 italic text-center",
                                "Fabric Mod Loader will install automatically. Please wait for the install to finish."
                            }
                            img {
                                src: "https://tallie.dev/modtool/assets/loader-slate-900.gif",
                                class: "mx-auto pt-6 w-1/2",
                            }
                        }
                    }
                }
            }
        }
    })
}

fn ForgeCheckPage(cx: Scope) -> Element {
    let check_complete = use_state(&cx, || false);
    let has_forge = use_state(&cx, || false);
    let forge_version = use_state(&cx, || String::from(""));

    let atoms = use_atom_root(&cx);
    let state = use_read(&cx, STATE);

    let mc_version = state.manifest.lookup(state.selected_profile).clone().meta.version;

    use_future(&cx, (), |_| forge_install(mc_version, has_forge.clone(), check_complete.clone(), forge_version.clone()));

/*    if *check_complete.get() && *has_forge.get() {
        let mut state_cpy = state.clone();
        state_cpy.page = Page::DownloadPage;
        atoms.set(STATE.unique_id(), state_cpy);
    }*/

    cx.render(rsx! {
        div {
            id: "forgecheckpage",
            class: "flex-1 flex-col flex justify-center w-full",
            match *check_complete.current() {
                true => { rsx! { 
                    h2 {
                        class: "text-6xl text-slate-100 mx-auto text-center font-bold",
                        "Check complete!"
                    },
                    div {
                        class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto flex-col w-1/2",
                        match *has_forge.get() {
                            true => rsx! {
                                div {
                                    class: "flex flex-col",
                                    p {
                                        class: "text-xl text-slate-300 mx-auto text-center",
                                        "Found Forge: "
                                    },
                                    p {
                                        class: "text-orange-600 font-bold text-xl text-center",
                                        "{forge_version}"
                                    },
                                    button {
                                        class: "bg-green-500 hover:bg-green-700 rounded-xl m-6 p-6 align-center mx-auto",
                                        onclick: move |_| {
                                            let mut state_cpy = state.clone();
                                            state_cpy.page = Page::DownloadPage;
                                            atoms.set(STATE.unique_id(), state_cpy);
                                        },
                                        img {
                                            src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                                            height: "32",
                                            width: "32",
                                            class: "mx-auto fill-slate-100"
                                        }
                                    }
                                }
                            },
                            false => rsx! {
                                div {
                                    class: "flex flex-col",
                                    p {
                                        class: "text-xl text-orange-600 font-bold text-center",
                                        "Could not install Forge!"
                                    },
                                    p {
                                        class: "text-xl text-slate-100 text-center",
                                        "Try re-running the program, or download and install Forge manually."
                                    },
                                    p {
                                        class: "text-sm text-slate-500 italic text-center",
                                        "Forge Mod Loader can be downloaded from ",
                                        a {
                                            class: "text-sm text-sky-500 italic underline",
                                            href: "https://files.minecraftforge.net/",
                                            "https://files.minecraftforge.net/"
                                        }
                                    },
                                    button {
                                        class: "bg-red-500 hover:bg-red-700 rounded-xl p-6 m-6 mb-0 mx-auto align-center",
                                        onclick: move |_| {
                                            let mut state_cpy = state.clone();
                                            state_cpy.page = Page::ProfilePage;
                                            atoms.set(STATE.unique_id(), state_cpy);
                                        },
                                        img {
                                            src: "https://tallie.dev/modtool/assets/fa-arrow-left.svg",
                                            height: "32",
                                            width: "32",
                                            class: "mx-auto fill-slate-100"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }},
                false => {
                    rsx! {
                        h2 {
                            class: "text-6xl text-slate-100 mx-auto text-center font-bold p-6",
                            "Installing Forge..."
                        },
                        div {
                            class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto flex-col w-1/2",
                            p {
                                class: "text-xl text-slate-100 font-bold text-center",
                                "Forge is now downloading."
                            },
                            p {
                                class: "text-xl text-slate-100 text-center",
                                "When the installer window appears, please install Forge normally."
                            },
                            p {
                                class: "text-sm text-slate-500 italic text-center",
                                "Select ⦿ Install client, click [OK], wait for the install to complete, then click [OK] to finish."
                            }
                            img {
                                src: "https://tallie.dev/modtool/assets/loader-slate-900.gif",
                                class: "mx-auto pt-6 w-1/2",
                            }
                        }
                    }
                }
            }
        }
    })

}

fn JavaCheckPage(cx: Scope) -> Element {
    let state = use_read(&cx, STATE);
    let atoms = use_atom_root(&cx);
    let has_java = use_state(&cx, || false);
    let check_complete = use_state(&cx, || false);
    let java_version = use_state(&cx, || String::from("1.8.023"));

    use_future(&cx, (), |_| { java_check(has_java.clone(), check_complete.clone(), java_version.clone()) } );

    if *check_complete.get() && *has_java.get() {
        let mut state_cpy = state.clone();
        state_cpy.page = Page::ProfilePage;
        atoms.set(STATE.unique_id(), state_cpy);
    }

    cx.render(rsx! {
        div {
            id: "completepage",
            class: "flex-1 flex-col flex justify-center w-full",
            style: "{{font-family: \"Bebas Neue\", sans-serif;}}",
            match *check_complete.get() {
                true => { rsx! { 
                    h2 {
                        class: "text-6xl text-slate-100 mx-auto text-center font-bold",
                        "Check complete!"
                    },
                    div {
                        class: "bg-slate-900 rounded-xl p-6 m-6 mx-auto flex-col w-1/2",
                        match *has_java.get() {
                            true => rsx! {
                                div {
                                    class: "flex flex-col",
                                    p {
                                        class: "text-xl text-slate-300 mx-auto text-center",
                                        "Found Java: "
                                    },
                                    p {
                                        class: "text-orange-600 font-bold text-xl text-center",
                                        "{java_version}"
                                    },
                                    button {
                                        class: "bg-green-500 hover:bg-green-700 rounded-xl m-6 p-6 align-center shrink flex-0",
                                        onclick: move |_| {
                                            let mut state_cpy = state.clone();
                                            state_cpy.page = Page::HomePage;
                                            atoms.set(STATE.unique_id(), state_cpy);
                                        },
                                        img {
                                            src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                                            height: "32",
                                            width: "32",
                                            class: "mx-auto fill-slate-100"
                                        }
                                    }
                                }
                            },
                            false => rsx! {
                                div {
                                    class: "flex flex-col",
                                    p {
                                        class: "text-xl text-orange-600 font-bold text-center",
                                        "Could not find Java!"
                                    },
                                    p {
                                        class: "text-xl text-slate-100 text-center",
                                        "Please download and install Java, then re-run this application."
                                    },
                                    p {
                                        class: "text-sm text-slate-500 italic text-center",
                                        "OpenJDK (Temurin) Java can be downloaded from ",
                                        a {
                                            class: "text-sm text-sky-500 italic underline",
                                            href: "https://adoptium.net/",
                                            "https://adoptium.net/"
                                        }
                                    },
                                    button {
                                        class: "bg-red-500 hover:bg-red-700 rounded-xl p-6 m-6 mb-0 mx-auto align-center",
                                        onclick: move |_| {
                                            std::process::exit(1);
                                        },
                                        img {
                                            src: "https://tallie.dev/modtool/assets/fa-xmark-circle.svg",
                                            height: "32",
                                            width: "32",
                                            class: "mx-auto fill-slate-100"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }},
                false => {
                    rsx! {
                        /*h2 {
                            class: "text-6xl text-slate-100 mx-auto text-center font-bold p-6",
                            "Checking for Java..."
                        },*/
                        img {
                            src: "https://tallie.dev/modtool/assets/loader-slate-800.gif",
                            class: "mx-auto",
                            width: "256px",
                            height: "256px"
                        }   
                    }
                }
            }
        }
    })
}

#[inline_props]
fn ProfileInfo(cx: Scope, meta: ProfileMeta) -> Element {

    let state = use_atom_state(&cx, STATE);
    let state_read = use_read(&cx, STATE);

    let mut text_primary_color = "text-slate-100";
    let mut text_secondary_color = "text-slate-300";
    let mut bg_primary_color = "bg-slate-700";
    let mut bg_secondary_color = "hover:bg-slate-600";
    let mut loader_accent_color = "text-sky-500";
    let mut loader_name = "Fabric";

    if meta.id == state.selected_profile {
        text_primary_color = "text-slate-900";
        text_secondary_color = "text-slate-600";
        bg_primary_color = "bg-slate-100";
        bg_secondary_color = "hover:bg-slate-300";
    }

    if meta.loader == ModLoader::Forge {
        loader_accent_color = "text-orange-300";
        loader_name = "Forge";
    }

    let mods_txt;
    let profiles_list = state.current().manifest.profiles.clone();
    let this_profile = profiles_list.clone()[meta.id as usize].clone();
    if this_profile.mods.len() == 0 {
        mods_txt = String::from("No mods.");
    } else if this_profile.mods.len() == 1 {
        mods_txt = this_profile.mods[0].name.clone();
    } else {
        mods_txt = this_profile.mods[0].name.clone() + format!(" and {} other mods", this_profile.mods.len() - 1).as_str();
    }

    let icon_url = match meta.loader {
        ModLoader::Fabric => "https://tallie.dev/modtool/assets/fa-scroll.svg",
        ModLoader::Forge => "https://tallie.dev/modtool/assets/fa-hammer.svg"
    };

    cx.render(rsx! {
        button {
            id: format_args!("profile_{}", meta.id),
            class: "{bg_primary_color} {bg_secondary_color} rounded-xl shrink {text_primary_color} p-6 text-left",
            onclick: move |_| {
                let mut state_cpy: AppState = state_read.clone();
                state_cpy.selected_profile = meta.id;
                state.with_mut(|st| {
                    st.selected_profile = meta.id;
                });
            },
            div {
                class: "flex flex-row flex-1",
               div {
                    class: "flex-1 flex flex-col",
                    h3 {
                        class: "text-3xl font-bold {text_primary_color}",
                        "{meta.name}"
                    },
                    p {
                        class: "text-xl {text_secondary_color}",
                        "{mods_txt}"
                    }
                },
                div {
                    class: "flex flex-col",
                    img {
                      src: "{icon_url}",
                      height: "48",
                      width: "48",
                      class: "mx-auto fill-slate-100 shrink"
                    },
                    p {
                        class: "{loader_accent_color} font-bold text-base",
                        "{loader_name} for {meta.version}"
                    }
                }
            }
        }
    })
}

fn DownloadPage(cx: Scope) -> Element {
    let ar = use_atom_root(&cx);
    let mut state = (*ar.read(STATE)).clone();

    let current_profile = state.manifest.lookup(state.selected_profile).clone();
    let mut mod_info_with_status = Vec::new();

   for modinfo in current_profile.mods.iter() {
        let modinfo = modinfo.clone();
        mod_info_with_status.push(ModDownload {
            name: modinfo.name,
            url: modinfo.url,
            version: modinfo.version,
            bytes_total: 0,
            bytes_read: 0,
            status: Download::InProgress,
            provider: modinfo.provider
        });
    };

    state.download_list = ModDownloads {
        downloads: mod_info_with_status.clone()
    };
    ar.set(STATE.unique_id(), state.clone());

    use_future(&cx, (), |_| { 
        let state = state.clone();
        async move {
            let current_profile = state.manifest.lookup(state.selected_profile).clone();

            let profiles_file = MC_DATA.base_dir.clone() + "launcher_profiles.json";
            let mut data = String::new(); 
			{
	            let mut file = File::open(profiles_file).unwrap();
		        file.read_to_string(&mut data).unwrap();
			}
            let mut profiles_json: LauncherProfiles = serde_json::from_str(&data).unwrap();

            let loader: &str = match current_profile.meta.loader {
                ModLoader::Fabric => "fabric",
                ModLoader::Forge => "forge",
            };

            let key = format!("modtool-rs-{}-{}-{}", loader, current_profile.meta.version, current_profile.meta.id);

			let mut version = String::from("");
			for file in std::fs::read_dir(MC_DATA.profiles_dir.clone()).unwrap() {
				let file = file.unwrap();
				let file_str = file.file_name().into_string().unwrap();
				if file_str.contains(loader) && file_str.contains(current_profile.meta.version.as_str()) {
					version = file.file_name().into_string().unwrap()
				}
			}

            if !profiles_json.profiles.contains_key(&key) {

                let date: DateTime<Utc> = Utc::now();
                let date_str: String = date.to_rfc3339_opts(SecondsFormat::Millis, true); 

                let profile = LauncherProfile {
                    created: Some(date_str),
                    lastUsed: String::from("1970-01-02T00:00:00.000Z"),
                    lastVersionId: version,
                    javaArgs: Some(String::from("-Dterminal.jline=false -Dterminal.ansi=true -XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1 -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:InitiatingHeapOccupancyPercent=15 -Dusing.aikars.flags=https://mcflags.emc.gs -Daikars.new.flags=true")),
                    name: format!("{} (ModTool RS)", current_profile.meta.name),
                    r#type: String::from("custom"),
                    icon: String::from("data:image/png;base64,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")
                };
                    
                profiles_json.profiles.insert(key, profile);

				let profiles_file = MC_DATA.base_dir.clone() + "launcher_profiles.json";
				let mut file = std::fs::OpenOptions::new().write(true).open(profiles_file).unwrap();

                match file.write(&serde_json::to_string_pretty(&profiles_json).unwrap().into_bytes()) {
					Ok(_) => {},
					Err(err) => println!("{}", err)
				};
            }
        }
    });

    let state = use_read(&cx, STATE);
    let current_profile = state.manifest.lookup(state.selected_profile).clone();
    let total_downloads = state.download_list.downloads.iter().len() as i32;
    let finished_downloads = use_state(&cx, || 0 as i32);
    let remaining_downloads = total_downloads - *finished_downloads.current();
    let atoms = use_atom_root(&cx);
    let mut sorted_state = state.download_list.downloads.clone();
    sorted_state.sort_by(|a, b| {
        if a.status == Download::InProgress && b.status == Download::InProgress { return Ordering::Equal };
        if a.status == Download::Complete && b.status == Download::Complete { return Ordering::Equal };
        if a.status == Download::InProgress && b.status == Download::Complete { return Ordering::Less };
        if a.status == Download::Complete && b.status == Download::InProgress { return Ordering::Greater };
        Ordering::Equal
    });

    cx.render(rsx! {
        div {
            id: "downloads",
            class: "flex flex-row flex-1 h-full",
            div {
                class: "overflow-y-auto flex-1",
                div {
                    class: "flex-1 flex flex-col pr-6 gap-6",
                    h2 {
                        class: "text-6xl font-bold text-slate-100 text-right",
                        "{current_profile.meta.name}"
                    }
                    div {
                        class: "jusify-end self-end justify-self-end flex flex-col gap-6",
                        sorted_state.iter().map(|modinfo| {
                            rsx! {
                                DownloadItem { modinfo: modinfo.clone() , downloads_complete: finished_downloads.clone()}
                            }
                        })
                    }
                },
            },
            div {
                class: "flex flex-col gap-6",
                div {
                    class: "flex-1 grow bg-slate-900 text-slate-100 p-6 rounded-xl text-sm text-left flex flex-col h-full overflow-y-auto align-stretch",
                    h3 {
                        class: "p-4 pl-0 gap-6",
                        "REMAINING"
                    },
                    p {
                        class: "text-huge rounded-xl font-bold text-center bg-sky-500 text-slate-100",
                        "{remaining_downloads}"
                    },
                    h3 {
                        class: "p-4 pl-0",
                        "DOWNLOADED"
                    },
                    p {
                        class: "text-huge rounded-xl bg-emerald-400 text-slate-100 font-bold text-center",
                        "{finished_downloads}"
                    },
                    h3 {
                        class: "p-4 pl-0",
                        "TOTAL"
                    },
                    p {
                        class: "text-huge font-bold text-center bg-slate-800 text-slate-100 rounded-xl",
                        "{total_downloads}"
                    },
                    div {
                        class: "grow flex-1"
                    },
                }
                match *finished_downloads.get() == total_downloads {
                    true => rsx! { 
                        button {
                            class: "bg-green-500 hover:bg-green-700 rounded-xl justify-self-end p-6 mt-auto self-center",
                            onclick: move |_| {
                                let mut state_cpy = state.clone();
                                state_cpy.page = Page::Complete;
                                atoms.set(STATE.unique_id(), state_cpy);
                            },
                            img {
                                src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                                height: "32",
                                width: "32",
                                class: "mx-auto fill-slate-100"
                            }
                        }
                    },
                    false => rsx! { 
                        button {
                            class: "bg-slate-800 rounded-xl justify-self-end p-6 mt-auto self-center",
                            img {
                                src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                                height: "32",
                                width: "32",
                                class: "mx-auto fill-slate-100"
                            }
                        }
                    }
                }
            }
        }
    })
}

#[inline_props]
fn DownloadItem(cx: Scope, modinfo: ModDownload, downloads_complete: UseState<i32>) -> Element {

    let download_state = use_state(&cx, || Download::InProgress);
    let downloaded_bytes = use_state(&cx, || 0 as u64);
    let total_bytes = use_state(&cx, || 1 as u64);
    let percentage = use_state(&cx, || 0 as u64);

    let ar = use_atom_root(&cx);

    use_future(&cx, (),  |_| {
        let downloads_complete = downloads_complete.clone();
        let download_state = download_state.clone();
        let mods_dir = MC_DATA.mods_dir.clone();
        let modinfo = modinfo.clone();
        let mut downloaded_bytes = downloaded_bytes.clone();
        let total_bytes = total_bytes.clone();
        let percentage = percentage.clone();

        let ar = ar.clone();

        async move {
            let sep = match cfg!(windows) {
                true => "\\",
                false => "/",
            };

            let path = Path::new(&modinfo.url);
			let filename = path.file_name().unwrap();
			let filepath = mods_dir.clone() + sep + filename.to_str().unwrap();
            let mut fhandle = File::create(filepath.clone()).unwrap();

            while *ar.read(NUM_DOWNLOADS) >= 4 {
                async_std::task::sleep(std::time::Duration::from_millis(100)).await;
            }

            ar.set(NUM_DOWNLOADS.unique_id(), *ar.read(NUM_DOWNLOADS) + 1);

            let res = HTTP_CLIENT
                .get(modinfo.url.clone())
                .header("User-Agent", format!("Starkiller645/modtool-rs/{APP_VERSION} (tallie@tallie.dev)"))
                .send()
                .await.unwrap();

            let content_length = res.content_length().unwrap().clone();
            total_bytes.set(content_length);

            let mut bytes_stream = res.bytes_stream();
            while let Some(item) = bytes_stream.next().await {
                let chunk = item.or(Err(format!("Error downloading file!"))).unwrap();
                fhandle.write_all(&chunk)
                    .unwrap();
                downloaded_bytes += chunk.len() as u64;
                percentage.set(((*downloaded_bytes.current() as f64 / *total_bytes.current() as f64) * 100.0) as u64);
            }

            ar.set(NUM_DOWNLOADS.unique_id(), *ar.read(NUM_DOWNLOADS) - 1);

            download_state.set(Download::Complete);
            download_state.needs_update();
            downloads_complete.set(*downloads_complete.current() + 1);
            downloads_complete.needs_update();
        }
    });


    let provider: String = match modinfo.provider {
        ModProvider::Modrinth => String::from("Modrinth"),
        ModProvider::CurseForge => String::from("CurseForge"),
        ModProvider::Creator => String::from("Creator's Website"),
        ModProvider::Unknown => String::from("Unknown")
    };

    let provider_color: String = match modinfo.provider {
        ModProvider::Modrinth => String::from("text-emerald-400"),
        ModProvider::CurseForge => String::from("text-orange-400"),
        ModProvider::Creator => String::from("text-cyan-400"),
        ModProvider::Unknown => String::from("text-slate-300")
    };

    let percentage_decimal = *downloaded_bytes.current() as f64 / *total_bytes.current() as f64;
    let percent = percentage_decimal * 100.0;
    let red = 252 - (percentage_decimal * 200.0) as u64;
    let green = 165 + (percentage_decimal * 46.0) as u64;
    let blue = 165 - (percentage_decimal * 12.0) as u64;

    let hex = format!("#{:X}{:X}{:X}", red, green, blue);

    let mb = format!("{:.2}MB", *total_bytes.current() as f64 / 1000000.0);

    cx.render(rsx! {
        div {
            class: "flex flex-row bg-slate-900 justify-end p-6 rounded-xl gap-6 flex-1 grow",
            div {
                class: "flex-1 grow flex-col",
                h3 {
                    class: "text-slate-100 text-2xl flex-1 grow font-bold text-right",
                    "{modinfo.name}"
                },
                p {
                    class: "text-slate-500 text-right",
                    "{modinfo.version}, from ",
                    span {
                        class: "{provider_color}",
                        "{provider}"
                    }
                },
                p {
                    class: "text-slate-500 font-bold text-right",
                    "{mb}"
                },
                div {
                    class: "bg-slate-800 rounded-xl text-slate-100 float-right w-full h-2",
                    div {
                        style: "width: {percent}%; background-color: {hex}; min-width: 0.5rem;",
                        class: "bg-orange-500 h-full rounded-xl",
                    }
                }
                p {
                    class: "text-slate-100 text-right font-bold py-2",
                    "{percentage}%"
                }
            }
            match *download_state.current() {
                Download::InProgress => rsx! {
                    img {
                        src: "https://tallie.dev/modtool/assets/loader-slate-900.gif",
                        height: "32",
                        width: "32",
                        class: "fill-slate-100 shrink ml-auto align-center object-scale-down"
                    }
                },
                Download::Complete => rsx! {
                    img {
                        src: "https://tallie.dev/modtool/assets/fa-check.svg",
                        height: "32",
                        width: "32",
                        class: "fill-orange-500 shrink ml-auto align-center"
                    }
                }
            }
        }
    })
}

fn ProfilePage(cx: Scope) -> Element {

    let state = use_read(&cx, STATE);
    let atoms = use_atom_root(&cx);

    cx.render(rsx! {
        div {
            id: "profile",
            class: "flex flex-row flex-1 h-full",
            div {
                class: "flex-1 rounded-xl text-slate-100 m-6 text-6xl font-bold flex flex-col p-6 gap-6 overflow-y-auto",
                "Profiles",
                    state.manifest.profiles.iter().map(|profile| {
                    let meta = profile.meta.clone();
                        rsx! {
                            ProfileInfo {
                                meta: meta
                        }
                    }})
            },
            button {
                onclick: move |_| {
                    match state.manifest.lookup(state.selected_profile).meta.loader { 
                    ModLoader::Forge =>  {
                        let mut state_cpy = state.clone();
                        state_cpy.page = Page::ForgeCheckPage;
                        atoms.set(STATE.unique_id(), state_cpy);
                    },
                    ModLoader::Fabric => {
                        let mut state_cpy = state.clone();
                        state_cpy.page = Page::FabricCheckPage;
                        atoms.set(STATE.unique_id(), state_cpy);
                    }
                }
                },
                class: "hover:bg-green-700 bg-green-500 rounded-xl p-6 m-6 grow-0 my-auto flex-0 shrink",
                img {
                  src: "https://tallie.dev/modtool/assets/fa-arrow-right.svg",
                  height: "32",
                  width: "32",
                  class: "mx-auto fill-slate-100"
                }
            }
        }
    })
}

fn ManifestPage(cx: Scope) -> Element {
    let atoms = use_atom_root(&cx);

    use_future(&cx, (), |_| manifest_download_handler(atoms.clone()));
    cx.spawn(
        manifest_download_handler(atoms.clone())
    );

    cx.render(rsx! {
        div {
              id: "manifestdl",
              class: "flex-1 flex-col flex justify-center",
              p { 
                  class: "text-6xl text-slate-500 text-center",
                  "Downloading manifest..."
              },  
              img {
                  src: "https://tallie.dev/modtool/assets/loader-slate-800.gif",
                  class: "mx-auto",
                  width: "256px",
                  height: "256px"
              }   
          }   
    })
}

async fn manifest_download_handler(ar: Rc<AtomRoot>) {
    let manifest_txt = reqwest::get("https://tallie.dev/modtool/manifest.json").await.unwrap()
        .text().await.unwrap();
    let manifest: Manifest = serde_json::from_str(manifest_txt.as_str()).unwrap();

    let mut new_manifest = Manifest {
        profiles: Vec::new()
    };

    new_manifest.profiles.push(Profile {
        meta: ProfileMeta {
            name: String::from("Test"),
            version: String::from("1.16.5"),
            loader: ModLoader::Fabric,
            id: 0
        },
        mods: Vec::new()
    });


    let mut state_cpy: AppState = (*ar.read(STATE)).clone();
    state_cpy.page = Page::HomePage;
    state_cpy.manifest = manifest.clone();
    ar.set(STATE.unique_id(), state_cpy);

}
