#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::*;
use std::fs::File;
use std::path::Path;
use std::io::Cursor;
use std::rc::Rc;
use std::env;
use serde::{Serialize, Deserialize};
use std::cmp::Ordering;
use lazy_static::*;

static APP_VERSION: &'static str = "2.0-alpha2";

#[derive(Copy, Clone)]
enum Page {
    ManifestDownloadPage,
    HomePage,
    ProfilePage,
    DownloadPage,
    Complete
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq)]
enum ModLoader {
    Fabric,
    Forge
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
    provider: ModProvider
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

struct MCData {
    base_dir: String,
    mods_dir: String,
    packs_dir: String,
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

static MOD_DOWNLOADS: Atom<ModDownloads> = |_| { ModDownloads {
    downloads: Vec::new()
}};

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
}

static MC_DATA: Atom<MCData> = |_| { 
    let base_dir = match cfg!(windows) {
        true => { env::var("APPDATA").unwrap() + "\\.minecraft\\"},
        false => { env::var("HOME").unwrap() + "/.minecraft/" }
    };
    MCData {
        base_dir: base_dir.to_string(),
        mods_dir: { base_dir.clone() + "mods" }.to_string(),
        profiles_dir: { base_dir.clone() + "versions" }.to_string(),
        packs_dir: { base_dir.clone() + "resourcepacks" }.to_string()
    }
};

fn main() {
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
                  " v1.0"
              }
            }
            button {
              onclick: move |_| {
                  match state.page {
                      Page::HomePage => {
                          let mut state_cpy = state.clone();
                          state_cpy.page = Page::ProfilePage;
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

/*async fn download_all_mod_files(ar: Rc<AtomRoot>) {
    let downloads_list = (*ar.read(MOD_DOWNLOADS)).clone();
    for download in downloads_list.downloads.iter() {
        async {
            let mut download = download.clone();
            thread::sleep(Duration::from_millis(2000));
            download.status = Download::Complete;
            ar.set(MOD_DOWNLOADS.unique_id(), downloads_list.clone());
        }.await;
        ar.force_update(MOD_DOWNLOADS.unique_id());
    }
}*/

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

    /*use_coroutine(&cx, |_: UnboundedReceiver<()>| {
        let ar = ar.clone();
        let state = (*ar.read(STATE)).clone();
        async move {
            let ar = ar.clone();
            let mut state: AppState = (*ar.read(STATE)).clone();
            let current_profile = state.manifest.lookup(state.selected_profile).clone();


            ar.set(STATE.unique_id(), state.clone());
            println!("State set complete");
            println!("Downloading mods");
			println!("{}", state.download_list.downloads.len());
            let mut download_list_copy = state.download_list.downloads.clone();
            
            let i: usize = 0;
            for download in state.download_list.downloads.iter() {
                println!("{}", download.name);
                thread::sleep(Duration::from_millis(1000));
                download_list_copy[i].status = Download::Complete;
                let mut current_state = (*ar.read(STATE)).clone();
                current_state.download_list = ModDownloads {
                    downloads: download_list_copy.clone()
                };
                ar.set(STATE.unique_id(), current_state.clone());
                let state_new = (*ar.read(STATE)).clone();
                match state_new.download_list.downloads[i].status {
                    Download::Complete => println!("Complete!"),
                    Download::InProgress => println!("Still in progress :("),
                }
                ar.force_update(STATE.unique_id());
            }
        }
    });*/

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
                class: "flex flex-col w-full gap-6",
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

    use_future(&cx, (),  |_| {
        let downloads_complete = downloads_complete.clone();
        let download_state = download_state.clone();
        let mc_data = use_atom_state(&cx, MC_DATA).clone();
        let modinfo = modinfo.clone();
        async move {
            let sep = match cfg!(windows) {
                true => "\\",
                false => "/",
            };

            let res = HTTP_CLIENT
                .get(modinfo.url.clone())
                .header("User-Agent", format!("Starkiller645/modtool-rs/{APP_VERSION} (tallie@tallie.dev)"))
                .send()
                .await.unwrap();
			let path = Path::new(&modinfo.url);
			let filename = path.file_name().unwrap();
			let filepath = mc_data.mods_dir.clone() + sep + filename.to_str().unwrap();
            let mut fhandle = File::create(filepath).unwrap();
			let mut content = Cursor::new(res.bytes().await.unwrap());
			std::io::copy(&mut content, &mut fhandle).unwrap();

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
                    let mut state_cpy = state.clone();
                    state_cpy.page = Page::DownloadPage;
                    atoms.set(STATE.unique_id(), state_cpy);
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
                  src: "https://tallie.dev/modtool/assets/loader.gif",
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
