//! FFI Win32 minimal yang dipakai proyek ini.
//!
//! Ditulis manual (tanpa crate `windows`) supaya tidak bentrok versi dengan
//! dependensi internal Tauri dan binary tetap kecil. Semua handle = `isize`.
#![allow(non_snake_case, non_camel_case_types, dead_code, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HANDLE = isize;
pub type HKEY = isize;
pub type HMONITOR = isize;
pub type HWINEVENTHOOK = isize;
pub type BOOL = i32;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ----- konstanta -----
pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
pub const WS_EX_APPWINDOW: isize = 0x0004_0000;
pub const WS_EX_NOACTIVATE: isize = 0x0800_0000;
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const SW_RESTORE: i32 = 9;
pub const HWND_TOPMOST: HWND = -1;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const GW_OWNER: u32 = 4;
pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
pub const MONITORINFOF_PRIMARY: u32 = 1;
pub const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
pub const EVENT_OBJECT_LOCATIONCHANGE: u32 = 0x800B;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0;
pub const WINEVENT_SKIPOWNPROCESS: u32 = 2;
pub const OBJID_WINDOW: i32 = 0;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_HOTKEY: u32 = 0x0312;
pub const WM_CLIPBOARDUPDATE: u32 = 0x031D;
pub const WM_POWERBROADCAST: u32 = 0x0218;
pub const PBT_POWERSETTINGCHANGE: usize = 0x8013;
pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001u32 as i32 as isize;
pub const KEY_READ: u32 = 0x0002_0019;
pub const KEY_SET_VALUE: u32 = 0x0002;
pub const REG_SZ: u32 = 1;
pub const REG_QWORD: u32 = 11;
pub const REG_NOTIFY_CHANGE_NAME: u32 = 0x1;
pub const REG_NOTIFY_CHANGE_LAST_SET: u32 = 0x4;
pub const INFINITE: u32 = 0xFFFF_FFFF;
pub const TH32CS_SNAPPROCESS: u32 = 0x2;
pub const PROCESS_SET_QUOTA: u32 = 0x0100;
pub const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
pub const INVALID_HANDLE_VALUE: HANDLE = -1;
pub const REG_DWORD: u32 = 4;
pub const PROCESS_SET_INFORMATION: u32 = 0x0200;
pub const PROCESS_TERMINATE: u32 = 0x0001;
pub const HIGH_PRIORITY_CLASS: u32 = 0x0000_0080;
pub const CF_UNICODETEXT: u32 = 13;
pub const GMEM_MOVEABLE: u32 = 0x0002;
pub const DWMWA_CLOAKED: u32 = 14;
pub const SHGFI_ICON: u32 = 0x100;
pub const SHGFI_LARGEICON: u32 = 0x0;
pub const ES_CONTINUOUS: u32 = 0x8000_0000;
pub const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
pub const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;
pub const ABM_GETSTATE: u32 = 0x4;
pub const ABM_SETSTATE: u32 = 0xA;
pub const ABS_AUTOHIDE: isize = 0x1;
pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_NOREPEAT: u32 = 0x4000;
pub const INPUT_KEYBOARD: u32 = 1;
pub const KEYEVENTF_KEYUP: u32 = 0x2;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_VOLUME_MUTE: u16 = 0xAD;

// ----- struct -----
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct MONITORINFO {
    pub cbSize: u32,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: usize,
    pub lParam: isize,
    pub time: u32,
    pub pt: POINT,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub struct MEMORYSTATUSEX {
    pub dwLength: u32,
    pub dwMemoryLoad: u32,
    pub ullTotalPhys: u64,
    pub ullAvailPhys: u64,
    pub ullTotalPageFile: u64,
    pub ullAvailPageFile: u64,
    pub ullTotalVirtual: u64,
    pub ullAvailVirtual: u64,
    pub ullAvailExtendedVirtual: u64,
}

#[repr(C)]
pub struct SYSTEM_POWER_STATUS {
    pub ACLineStatus: u8,
    pub BatteryFlag: u8,
    pub BatteryLifePercent: u8,
    pub SystemStatusFlag: u8,
    pub BatteryLifeTime: u32,
    pub BatteryFullLifeTime: u32,
}

#[repr(C)]
pub struct PROCESSENTRY32W {
    pub dwSize: u32,
    pub cntUsage: u32,
    pub th32ProcessID: u32,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: u32,
    pub cntThreads: u32,
    pub th32ParentProcessID: u32,
    pub pcPriClassBase: i32,
    pub dwFlags: u32,
    pub szExeFile: [u16; 260],
}

#[repr(C)]
pub struct PROCESS_MEMORY_COUNTERS {
    pub cb: u32,
    pub PageFaultCount: u32,
    pub PeakWorkingSetSize: usize,
    pub WorkingSetSize: usize,
    pub QuotaPeakPagedPoolUsage: usize,
    pub QuotaPagedPoolUsage: usize,
    pub QuotaPeakNonPagedPoolUsage: usize,
    pub QuotaNonPagedPoolUsage: usize,
    pub PagefileUsage: usize,
    pub PeakPagefileUsage: usize,
}

pub type WNDPROC = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;
pub type WNDENUMPROC = unsafe extern "system" fn(HWND, isize) -> BOOL;
pub type WINEVENTPROC = unsafe extern "system" fn(HWINEVENTHOOK, u32, HWND, i32, i32, u32, u32);

#[repr(C)]
pub struct WNDCLASSW {
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: isize,
    pub hIcon: isize,
    pub hCursor: isize,
    pub hbrBackground: isize,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
pub struct ICONINFO {
    pub fIcon: BOOL,
    pub xHotspot: u32,
    pub yHotspot: u32,
    pub hbmMask: isize,
    pub hbmColor: isize,
}

#[repr(C)]
pub struct BITMAP {
    pub bmType: i32,
    pub bmWidth: i32,
    pub bmHeight: i32,
    pub bmWidthBytes: i32,
    pub bmPlanes: u16,
    pub bmBitsPixel: u16,
    pub bmBits: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

#[repr(C)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 1],
}

#[repr(C)]
pub struct SHFILEINFOW {
    pub hIcon: isize,
    pub iIcon: i32,
    pub dwAttributes: u32,
    pub szDisplayName: [u16; 260],
    pub szTypeName: [u16; 80],
}

#[repr(C)]
pub struct APPBARDATA {
    pub cbSize: u32,
    pub hWnd: HWND,
    pub uCallbackMessage: u32,
    pub uEdge: u32,
    pub rc: RECT,
    pub lParam: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KEYBDINPUT {
    pub wVk: u16,
    pub wScan: u16,
    pub dwFlags: u32,
    pub time: u32,
    pub dwExtraInfo: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MOUSEINPUT {
    pub dx: i32,
    pub dy: i32,
    pub mouseData: u32,
    pub dwFlags: u32,
    pub time: u32,
    pub dwExtraInfo: usize,
}

#[repr(C)]
pub union INPUT_U {
    pub mi: MOUSEINPUT,
    pub ki: KEYBDINPUT,
}

#[repr(C)]
pub struct INPUT {
    pub typ: u32,
    pub u: INPUT_U,
}

// ----- fungsi -----
#[link(name = "user32")]
extern "system" {
    pub fn GetForegroundWindow() -> HWND;
    pub fn SetForegroundWindow(hwnd: HWND) -> BOOL;
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    pub fn IsIconic(hwnd: HWND) -> BOOL;
    pub fn GetWindow(hwnd: HWND, cmd: u32) -> HWND;
    pub fn GetWindowLongW(hwnd: HWND, index: i32) -> i32;
    pub fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    pub fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    pub fn GetClassNameW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetWindowThreadProcessId(hwnd: HWND, pid: *mut u32) -> u32;
    pub fn EnumWindows(cb: WNDENUMPROC, lparam: isize) -> BOOL;
    pub fn MonitorFromWindow(hwnd: HWND, flags: u32) -> HMONITOR;
    pub fn MonitorFromPoint(pt: POINT, flags: u32) -> HMONITOR;
    pub fn GetMonitorInfoW(mon: HMONITOR, info: *mut MONITORINFO) -> BOOL;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> BOOL;
    pub fn SetWindowPos(hwnd: HWND, after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> BOOL;
    pub fn SetWindowRgn(hwnd: HWND, rgn: isize, redraw: BOOL) -> i32;
    pub fn SetWinEventHook(
        min: u32,
        max: u32,
        module: isize,
        proc_: Option<WINEVENTPROC>,
        pid: u32,
        tid: u32,
        flags: u32,
    ) -> HWINEVENTHOOK;
    pub fn UnhookWinEvent(hook: HWINEVENTHOOK) -> BOOL;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> BOOL;
    pub fn TranslateMessage(msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(msg: *const MSG) -> isize;
    pub fn PostThreadMessageW(tid: u32, msg: u32, w: usize, l: isize) -> BOOL;
    pub fn PostMessageW(hwnd: HWND, msg: u32, w: usize, l: isize) -> BOOL;
    pub fn PostQuitMessage(code: i32);
    pub fn RegisterClassW(wc: *const WNDCLASSW) -> u16;
    pub fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: isize,
        inst: isize,
        param: *const c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize;
    pub fn DestroyWindow(hwnd: HWND) -> BOOL;
    pub fn AddClipboardFormatListener(hwnd: HWND) -> BOOL;
    pub fn RemoveClipboardFormatListener(hwnd: HWND) -> BOOL;
    pub fn OpenClipboard(hwnd: HWND) -> BOOL;
    pub fn CloseClipboard() -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn GetClipboardData(fmt: u32) -> HANDLE;
    pub fn SetClipboardData(fmt: u32, h: HANDLE) -> HANDLE;
    pub fn IsClipboardFormatAvailable(fmt: u32) -> BOOL;
    pub fn RegisterClipboardFormatW(name: *const u16) -> u32;
    pub fn RegisterHotKey(hwnd: HWND, id: i32, mods: u32, vk: u32) -> BOOL;
    pub fn UnregisterHotKey(hwnd: HWND, id: i32) -> BOOL;
    pub fn RegisterPowerSettingNotification(h: HANDLE, guid: *const GUID, flags: u32) -> HANDLE;
    pub fn UnregisterPowerSettingNotification(h: HANDLE) -> BOOL;
    pub fn PrivateExtractIconsW(
        file: *const u16,
        index: i32,
        cx: i32,
        cy: i32,
        icons: *mut isize,
        ids: *mut u32,
        count: u32,
        flags: u32,
    ) -> u32;
    pub fn GetIconInfo(icon: isize, info: *mut ICONINFO) -> BOOL;
    pub fn DestroyIcon(icon: isize) -> BOOL;
    pub fn SendInput(count: u32, inputs: *const INPUT, size: i32) -> u32;
    pub fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
    pub fn LockWorkStation() -> BOOL;
    pub fn SendMessageW(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> isize;
    pub fn CreateCompatibleDC(hdc: isize) -> isize;
    pub fn DeleteDC(hdc: isize) -> BOOL;
    pub fn DeleteObject(obj: isize) -> BOOL;
    pub fn GetObjectW(h: isize, size: i32, out: *mut c_void) -> i32;
    pub fn GetDIBits(
        hdc: isize,
        hbm: isize,
        start: u32,
        lines: u32,
        bits: *mut c_void,
        bi: *mut BITMAPINFO,
        usage: u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> isize;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetTickCount64() -> u64;
    pub fn GlobalMemoryStatusEx(status: *mut MEMORYSTATUSEX) -> BOOL;
    pub fn GetSystemTimes(idle: *mut FILETIME, kernel: *mut FILETIME, user: *mut FILETIME) -> BOOL;
    pub fn GetSystemPowerStatus(status: *mut SYSTEM_POWER_STATUS) -> BOOL;
    pub fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> BOOL;
    pub fn SetThreadExecutionState(flags: u32) -> u32;
    pub fn GlobalAlloc(flags: u32, bytes: usize) -> HANDLE;
    pub fn GlobalLock(h: HANDLE) -> *mut c_void;
    pub fn GlobalUnlock(h: HANDLE) -> BOOL;
    pub fn QueryFullProcessImageNameW(h: HANDLE, flags: u32, buf: *mut u16, size: *mut u32) -> BOOL;
    pub fn CreateEventW(attrs: *const c_void, manual: BOOL, initial: BOOL, name: *const u16) -> HANDLE;
    pub fn SetEvent(h: HANDLE) -> BOOL;
    pub fn WaitForMultipleObjects(count: u32, handles: *const HANDLE, wait_all: BOOL, ms: u32) -> u32;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn OpenProcess(access: u32, inherit: BOOL, pid: u32) -> HANDLE;
    pub fn K32EmptyWorkingSet(process: HANDLE) -> BOOL;
    pub fn K32GetProcessMemoryInfo(process: HANDLE, counters: *mut PROCESS_MEMORY_COUNTERS, cb: u32) -> BOOL;
    pub fn SetPriorityClass(process: HANDLE, class: u32) -> BOOL;
    pub fn TerminateProcess(process: HANDLE, code: u32) -> BOOL;
    pub fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> HANDLE;
    pub fn Process32FirstW(snap: HANDLE, entry: *mut PROCESSENTRY32W) -> BOOL;
    pub fn Process32NextW(snap: HANDLE, entry: *mut PROCESSENTRY32W) -> BOOL;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(key: HKEY, sub: *const u16, options: u32, sam: u32, out: *mut HKEY) -> i32;
    pub fn RegCreateKeyExW(
        key: HKEY,
        sub: *const u16,
        reserved: u32,
        class: *const u16,
        options: u32,
        sam: u32,
        security: *const c_void,
        out: *mut HKEY,
        disposition: *mut u32,
    ) -> i32;
    pub fn RegSetValueExW(key: HKEY, name: *const u16, reserved: u32, typ: u32, data: *const u8, len: u32) -> i32;
    pub fn RegDeleteValueW(key: HKEY, name: *const u16) -> i32;
    pub fn RegCloseKey(key: HKEY) -> i32;
    pub fn RegNotifyChangeKeyValue(key: HKEY, subtree: BOOL, filter: u32, event: HANDLE, asynchronous: BOOL) -> i32;
    pub fn RegEnumKeyExW(
        key: HKEY,
        index: u32,
        name: *mut u16,
        name_len: *mut u32,
        reserved: *mut u32,
        class: *mut u16,
        class_len: *mut u32,
        last_write: *mut FILETIME,
    ) -> i32;
    pub fn RegQueryValueExW(
        key: HKEY,
        name: *const u16,
        reserved: *mut u32,
        typ: *mut u32,
        data: *mut u8,
        len: *mut u32,
    ) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn ShellExecuteW(
        hwnd: HWND,
        op: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> isize;
    pub fn SHGetFileInfoW(path: *const u16, attrs: u32, info: *mut SHFILEINFOW, size: u32, flags: u32) -> usize;
    pub fn SHAppBarMessage(msg: u32, data: *mut APPBARDATA) -> usize;
}

#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmGetWindowAttribute(hwnd: HWND, attr: u32, out: *mut c_void, size: u32) -> i32;
}

#[link(name = "iphlpapi")]
extern "system" {
    pub fn GetIfTable(table: *mut u8, size: *mut u32, order: BOOL) -> u32;
    pub fn IcmpCreateFile() -> HANDLE;
    pub fn IcmpCloseHandle(h: HANDLE) -> BOOL;
    pub fn IcmpSendEcho(
        h: HANDLE,
        dest: u32,
        data: *const c_void,
        data_size: u16,
        options: *const c_void,
        reply: *mut c_void,
        reply_size: u32,
        timeout_ms: u32,
    ) -> u32;
}
