//! Ekstraksi ikon file/aplikasi → PNG (data URL). Dipakai dock, shelf, dan pintasan.
//! Dijalankan di thread-pool (command async) dan di-cache.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::sys::*;

static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

pub fn icon_data_url(path: &str) -> Option<String> {
    {
        let c = CACHE.lock().unwrap();
        if let Some(hit) = c.as_ref().and_then(|m| m.get(path)) {
            return hit.clone();
        }
    }
    let res = extract(path);
    CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(path.to_string(), res.clone());
    res
}

fn extract(path: &str) -> Option<String> {
    unsafe {
        let wp = wide(path);
        let lower = path.to_lowercase();
        let mut hicon: isize = 0;

        // 1) Ikon resolusi tinggi dari exe/ico/dll.
        if lower.ends_with(".exe") || lower.ends_with(".ico") || lower.ends_with(".dll") {
            let mut id = 0u32;
            let n = PrivateExtractIconsW(wp.as_ptr(), 0, 128, 128, &mut hicon, &mut id, 1, 0);
            if n == 0 || n == 0xFFFF_FFFF {
                hicon = 0;
            }
        }
        // 2) Fallback shell: apa pun (lnk, folder, dokumen, UWP).
        if hicon == 0 {
            let mut info: SHFILEINFOW = std::mem::zeroed();
            let r = SHGetFileInfoW(
                wp.as_ptr(),
                0,
                &mut info,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if r == 0 || info.hIcon == 0 {
                return None;
            }
            hicon = info.hIcon;
        }
        let png = hicon_to_png(hicon);
        DestroyIcon(hicon);
        png.map(|b| format!("data:image/png;base64,{}", STANDARD.encode(b)))
    }
}

unsafe fn read_bits(hdc: isize, hbm: isize, w: i32, h: i32) -> Option<Vec<u8>> {
    let mut bi: BITMAPINFO = std::mem::zeroed();
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = w;
    bi.bmiHeader.biHeight = -h; // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    let lines = GetDIBits(hdc, hbm, 0, h as u32, px.as_mut_ptr() as *mut c_void, &mut bi, 0);
    (lines != 0).then_some(px)
}

unsafe fn hicon_to_png(hicon: isize) -> Option<Vec<u8>> {
    let mut ii: ICONINFO = std::mem::zeroed();
    if GetIconInfo(hicon, &mut ii) == 0 {
        return None;
    }
    let result = (|| {
        if ii.hbmColor == 0 {
            return None;
        }
        let mut bm: BITMAP = std::mem::zeroed();
        GetObjectW(ii.hbmColor, std::mem::size_of::<BITMAP>() as i32, &mut bm as *mut _ as *mut c_void);
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 512 || h > 512 {
            return None;
        }
        let hdc = CreateCompatibleDC(0);
        let color = read_bits(hdc, ii.hbmColor, w, h);
        let has_alpha = color.as_ref().map(|c| c.chunks(4).any(|p| p[3] != 0)).unwrap_or(false);
        let mask = if has_alpha || ii.hbmMask == 0 { None } else { read_bits(hdc, ii.hbmMask, w, h) };
        DeleteDC(hdc);

        let mut px = color?;
        for (i, p) in px.chunks_mut(4).enumerate() {
            p.swap(0, 2); // BGRA → RGBA
            if !has_alpha {
                let transparent = mask.as_ref().map(|m| m[i * 4] != 0).unwrap_or(false);
                p[3] = if transparent { 0 } else { 255 };
            }
        }

        // 128 → 64 (box filter): hemat memori, tetap tajam untuk dock.
        let (mut ow, mut oh, mut data) = (w as u32, h as u32, px);
        if ow >= 128 {
            let (nw, nh) = (ow / 2, oh / 2);
            let mut out = vec![0u8; (nw * nh * 4) as usize];
            for y in 0..nh {
                for x in 0..nw {
                    for c in 0..4 {
                        let mut s = 0u32;
                        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            s += data[(((y * 2 + dy) * ow + x * 2 + dx) * 4 + c) as usize] as u32;
                        }
                        out[((y * nw + x) * 4 + c) as usize] = (s / 4) as u8;
                    }
                }
            }
            data = out;
            ow = nw;
            oh = nh;
        }

        let mut buf = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut buf, ow, oh);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().ok()?;
            wr.write_image_data(&data).ok()?;
        }
        Some(buf)
    })();
    if ii.hbmColor != 0 {
        DeleteObject(ii.hbmColor);
    }
    if ii.hbmMask != 0 {
        DeleteObject(ii.hbmMask);
    }
    result
}
