#![no_main]
#![no_std]

use core::arch::asm;
use core::ffi::c_void;
use core::ptr::null;
use log::info;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat};
use uefi::{prelude::*, CStr16, Identify};
use uefi::proto::console::text::Color;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::{AllocateType, MemoryType, SearchType};

pub mod utils;
pub mod gop;
use gop::*;

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    // とりあえず初期化。
    // ＴＯＤＯ：この汚いコードを書き換えたい。
    let mut gop_info = GopInfo {
        frame_buffer: null(),
        holizontal_resolution: 0,
        vertical_resolution: 0,
        stride: 0,
        pixel_format: FfiPixelFormat::Rgb,
    };

    // system_tableをexit_boot_servicesでmoveできるようにくくってある。
    // もっといい書き方を探し中。
    {
        // 画面の初期設定を行う
        let _ = system_table.stdout().set_color(Color::White, Color::Black);
        let _ = system_table.stdout().clear();
        let _ = system_table.stdout().enable_cursor(true);

        info!("Hello world!");

        // メモリマップを取得する
        let mut mem_map_buf = [0u8; 0x4000];
        let _memory_map = match system_table.boot_services().memory_map(&mut mem_map_buf) {
            Err(_) => panic!("Failed to get memory map."),
            Ok(memory_map) => memory_map,
        };
        
        // メモリマップの各エントリを出力する
        // for entry in memory_map.entries() {
        //     info!("{entry:?}");
        // }

        let loaded_image = system_table
            .boot_services()
            .open_protocol_exclusive::<LoadedImage>(image_handle)
            .unwrap();

        let mut fs = system_table
            .boot_services()
            .open_protocol_exclusive::<SimpleFileSystem>(loaded_image.device().unwrap())
            .unwrap();

        // ファイルシステムのrootディレクトリを取得し、rootディレクトリに含まれる
        // エントリを走査する。
        let mut root_dir = fs.open_volume().unwrap();
        let mut dentry_buf = [0u8; 0x1000]; // FileInfo用のバッファ
        while let Some(file_info) = root_dir.read_entry(&mut dentry_buf).unwrap() {
            info!("file_name: {}", file_info.file_name());
        }

        // rootディレクトリに置いてあるkernel.elfファイルをopenする
        // UEFIは文字列をCStr16で扱っているので、そのデータ変換も必要であるk
        // openしたファイルはFileHandleという型で返される。
        let kernel_file_name_str = "\\kernel.elf";
        let mut kernel_file_name_buf = [0u16; 0x100];
        let kernel_file_name_wide = CStr16::from_str_with_buf(kernel_file_name_str, &mut kernel_file_name_buf).unwrap();
        let mut kernel_file = root_dir.open(
            kernel_file_name_wide,
            FileMode::Read,
            FileAttribute::empty()).unwrap();
        
        // ファイルの各情報を取得する。
        let mut kernel_file_info_buf = [0u8; 0x400];
        let kernel_file_info = kernel_file.get_info::<FileInfo>(&mut kernel_file_info_buf).unwrap();
        let mut kernel_file_size = kernel_file_info.file_size() as usize;
        kernel_file_size = ((kernel_file_size + 0xfff) >> 12) << 12;
        info!("kernel file name: {}", kernel_file_info.file_name());
        info!("kernel file size: {}", kernel_file_size);
        info!("kernel physical size: {}", kernel_file_info.physical_size());

        // 0x100000にカーネルイメージをベタバリする。
        let ty = AllocateType::Address(0x100000);
        let mem_ty = MemoryType::LOADER_DATA;
        let kernel_file_base = system_table
            .boot_services()
            .allocate_pages(ty, mem_ty, kernel_file_size >> 12)
            .unwrap();

        // カーネルを読み出す先のメモリ領域をスライスにして管理する
        let kernel_file_base_ptr = kernel_file_base as *mut u8; 
        let kernel_file_slice = unsafe {
            core::slice::from_raw_parts_mut(kernel_file_base_ptr, kernel_file_size) };
        
        // readとかができるのはRegularFile型なので、型の変換をする。
        let mut kernel_file = kernel_file.into_regular_file().unwrap();
        let size = kernel_file.read(kernel_file_slice).unwrap();
        info!("write {size} byte to 0x100000");
        info!("{:?}", &kernel_file_slice[0..0x100]);
        utils::hexdump(&kernel_file_slice[0..0x100]);
        info!("{:p}", &kernel_file_slice[0] as *const u8);

        // ファイルは読み出しが成功したので、closeしておく。
        kernel_file.close();
        
        let gop_handles = system_table
            .boot_services()
            .locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))
            .unwrap();

        // 何故かQEMUだとGOPが2つ見つかり、1つ目のGOPをopenするとripがさまよってしまう、、。
        // とりあえず、gop_handlesの後ろから取ってくればうまくいったので、これでどうにかしようと思う。
        info!("gop_handles.len(): {}", gop_handles.len());
        let mut gop = system_table
            .boot_services()
            .open_protocol_exclusive::<GraphicsOutput>(*gop_handles.last().unwrap())
            .unwrap();

        let mode_info = gop.current_mode_info();
        info!("resolution: {:?}", mode_info.resolution());
        info!("pixel format: {:?}", mode_info.pixel_format());
        info!("stride: {:?}", mode_info.stride());

        let mut frame_buffer = gop.frame_buffer();
        let frame_buffer_ptr = frame_buffer.as_mut_ptr();
        info!("frame_buffer: {:?}", frame_buffer);
        info!("frame_buffer_ptr: {:p}", frame_buffer_ptr);
        
        gop_info.frame_buffer = frame_buffer.as_mut_ptr() as *const c_void;
        gop_info.holizontal_resolution = mode_info.resolution().0;
        gop_info.vertical_resolution = mode_info.resolution().1;
        gop_info.stride = mode_info.stride();
        gop_info.pixel_format = match mode_info.pixel_format() {
            PixelFormat::Rgb => FfiPixelFormat::Rgb,
            PixelFormat::Bgr => FfiPixelFormat::Bgr,
            pf => panic!("Undefined Pixed Format {:?}", pf)
        };
    }

    info!("gop_info.frame_buffer: {:p}", gop_info.frame_buffer);
    info!("gop_info.holizontal_resolution: {}", gop_info.holizontal_resolution);
    info!("gop_info.vertical_resolution: {}", gop_info.vertical_resolution);
    info!("gop_info.stride: {}", gop_info.stride);

    clear_screen(&gop_info);

    // ＴＯＤＯ：kernelに渡すboot parameterをここでいろいろと読み出しておく必要があるが、、、
    
    // EXIT_BOOT_SERVICESを発行する。
    // この関数内部で再度memory_mapを取得し、その最新のmap_kerが使われるので、
    // 失敗することはない。引数には、その取得するmemory_mapを配置するメモリ領域の
    // typeを指定するが、カーネルブートと同時に必要なくなるので、
    // BOOT_SERVICES_DATA領域としてマップするようにした。
    let (_system_table_runtime, _memory_map) = system_table.exit_boot_services(MemoryType::BOOT_SERVICES_DATA);

    // カーネルのエントリポイントへジャンプ！！
    unsafe {
        asm!("mov rdi, {}", in(reg) &gop_info as *const GopInfo as *const u8);
        asm!("jmp {}", in(reg) 0x100000 as *const u8, options(noreturn));
    };
}
