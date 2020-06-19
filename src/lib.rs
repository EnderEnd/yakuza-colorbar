use std::ffi::{CStr, CString};
use std::io;
use std::os::raw::c_char;
use std::ptr;
use winapi;
use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID};
use winapi::um::consoleapi::AllocConsole;
use std::collections::HashMap;
use ini::Ini;
use hex;

#[derive(PartialEq, Debug, Hash, Eq)]
enum Style {
    BrawlerFirst,
    Brawler,
    Beast,
    Rush,
    Legend
}

#[derive(Debug, Default)]
struct ColorBar {
    addr_charged: usize,
    color_charged: Vec<u8>,

    addr_uncharged: usize,
    color_uncharged: Vec<u8>
}


fn initialize_colors() -> HashMap<Style, ColorBar> {
    let mut colorbars = HashMap::new();

    colorbars.insert(Style::BrawlerFirst, ColorBar {
        addr_charged: 0xEE914,
        ..Default::default()
    });

    colorbars.insert(Style::Brawler, ColorBar {
        addr_charged: 0xEEA3A,
        addr_uncharged: 0xEE996,
        ..Default::default()
    });

    colorbars.insert(Style::Beast, ColorBar {
        addr_charged: 0xEE920,
        addr_uncharged: 0xEE97A,
        ..Default::default()
    });

    colorbars.insert(Style::Rush, ColorBar {
        addr_charged: 0xEE926,
        addr_uncharged: 0xEE988,
        ..Default::default()
    });

    colorbars.insert(Style::Legend, ColorBar {
        addr_charged: 0xEE91A,
        addr_uncharged: 0xEE96C,
        ..Default::default()
    });

    colorbars
}

fn write_aob(addr: usize, data: Vec<u8>) {
    use winapi::um::memoryapi::VirtualProtect;
    let s = data.len();
    let mut prot: DWORD = 0x0;
    let mut ptr = addr;
    unsafe {
        VirtualProtect(
            addr as LPVOID,
            s,
            winapi::um::winnt::PAGE_EXECUTE_READWRITE,
            &mut prot,
        );
        let mut target = ptr as *mut u8;
        for x in data {
            *target = x;
            ptr += 1;
            target = ptr as *mut u8;
        }
        VirtualProtect(addr as LPVOID, s, prot, std::ptr::null_mut());
    }
}

fn spit_err(body: &str) {
    let t = CString::new("color_injector").unwrap();
    let b = CString::new(body).unwrap();
    unsafe {
        winapi::um::winuser::MessageBoxA(std::ptr::null_mut(), b.as_ptr(), t.as_ptr(), 0x10);
    }
}

fn parse_ini() -> Result<HashMap<Style, ColorBar>, ini::ini::Error> {
    let mut parsed = HashMap::new();
    let conf = Ini::load_from_file("colors.ini").map_err(|error| {
        spit_err("colors.ini was not found in the exe folder");
        return error;
    })?;

    macro_rules! load_values {
        ($orig:expr, $dest:expr, $sec:expr) => {{
            let s_charged = $dest
                .get("charged")
                .unwrap();
            let charged = hex::decode(s_charged).unwrap();

            let s_uncharged = $dest
                .get("uncharged")
                .unwrap();
            let uncharged = hex::decode(s_uncharged).unwrap();

            parsed.insert($orig, ColorBar {
                color_charged: charged.clone(),
                color_uncharged: uncharged.clone(),
                ..Default::default()
            });

            if $orig == Style::Brawler {
                parsed.insert(Style::BrawlerFirst, ColorBar { color_charged: charged.clone(), ..Default::default() });
            }
        }};

    }
    for (sec, prop) in conf.iter() {
        match sec {
            Some("Brawler") => load_values!(Style::Brawler, prop, sec.unwrap()),
            Some("Beast") => load_values!(Style::Beast, prop, sec.unwrap()),
            Some("Rush") => load_values!(Style::Rush, prop, sec.unwrap()),
            Some("Legend") => load_values!(Style::Legend, prop, sec.unwrap()),
            _ => spit_err(&format!("{} is not a recognized Section", sec.unwrap()))
        };
    }

    Ok(parsed)
}

fn write_data(colors: &mut HashMap<Style, ColorBar>, mba: usize) -> Result<(), ini::ini::Error> {
    let parsed = parse_ini()?;

    for (style, col) in &parsed {
        match colors.get_mut(style) {
            Some(_col) => {
                if col.color_charged.len() > 0 && _col.addr_charged != 0 {
                    write_aob(mba + _col.addr_charged, col.color_charged.clone());
                }

                if col.color_uncharged.len() > 0 && _col.addr_uncharged != 0 {
                    write_aob(mba + _col.addr_uncharged, col.color_uncharged.clone());
                }
            },
            None => (),
        };
    }

    Ok(())

}

#[no_mangle]
pub unsafe extern "system" fn init(_: LPVOID) -> DWORD {
    let mut _buff = String::new();

    let game_name = CString::new("Yakuza0.exe").unwrap();
    let mba = winapi::um::libloaderapi::GetModuleHandleA(game_name.as_ptr()) as usize;

    let mut colors = initialize_colors();
    match write_data(&mut colors, mba) {
        Ok(_) => (),
        Err(_) => return 0
    }

    return 1;
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(_: HINSTANCE, reason: DWORD, _: LPVOID) -> BOOL {
    unsafe {
        match reason {
            winapi::um::winnt::DLL_PROCESS_ATTACH => {
                winapi::um::processthreadsapi::CreateThread(
                    ptr::null_mut(),
                    0,
                    Some(init),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                );
            }
            _ => (),
        };
    }

    return true as BOOL;
}
