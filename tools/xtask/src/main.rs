#![forbid(unsafe_code)]
//! Helper commands for local TXPOS development.


use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const DIST_DIR: &str = "dist";
const ESP_DIR: &str = "dist/esp";
const PARTITION_START_LBA: u32 = 2048;
const FAT_VOLUME_SECTORS: u32 = 131_072;
const BYTES_PER_SECTOR: usize = 512;
const ISO_SECTOR_SIZE: usize = 2048;
const RESERVED_SECTORS: u32 = 32;
const FAT_COUNT: u32 = 2;
const SECTORS_PER_CLUSTER: u32 = 1;

type XtaskResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> XtaskResult<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "check-env" => {
            check_env();
            Ok(())
        }
        "milestone" => {
            milestone();
            Ok(())
        }
        "build-kernel" => build_kernel(),
        "build-uefi" => build_uefi(),
        "stage-esp" => stage_esp(),
        "build-img" => build_img(),
        "build-iso" => build_iso(),
        "build-release-image" => build_release_image(),
        _ => {
            help();
            Ok(())
        }
    }
}

fn help() {
    println!("TXPOS xtask");
    println!("  cargo run -p xtask -- check-env");
    println!("  cargo run -p xtask -- milestone");
    println!("  cargo run -p xtask -- build-kernel");
    println!("  cargo run -p xtask -- build-uefi");
    println!("  cargo run -p xtask -- stage-esp");
    println!("  cargo run -p xtask -- build-img");
    println!("  cargo run -p xtask -- build-iso");
    println!("  cargo run -p xtask -- build-release-image");
}

fn milestone() {
    println!("TXPOS milestone 1: UEFI boot image groundwork");
    println!(
        "Implemented: memory, scheduler, security policy, txshield, txsentinel, txvault contracts, txfirewall, txfs metadata, networking parsers"
    );
    println!(
        "Added: UEFI bootloader entry, bare-metal kernel entry, ESP staging, FAT32 IMG builder, UEFI ISO wrapper"
    );
    println!("Next: install Rust UEFI/bare-metal targets and run build-release-image");
}

fn check_env() {
    report("rustc", &["--version"]);
    report("cargo", &["--version"]);
    report("rustup", &["target", "list", "--installed"]);
    report("qemu-system-x86_64", &["--version"]);
    report("VBoxManage", &["--version"]);
}

fn report(program: &str, args: &[&str]) {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            println!("{program}: {}", text.trim());
        }
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stderr);
            println!("{program}: unavailable ({})", text.trim());
        }
        Err(error) => println!("{program}: unavailable ({error})"),
    }
}

fn build_release_image() -> XtaskResult<()> {
    run_command("cargo", &["test", "--workspace"])?;
    build_kernel()?;
    build_uefi()?;
    stage_esp()?;
    build_img()?;
    build_iso()?;
    println!("release image complete: dist/txpos.iso");
    Ok(())
}

static KERNEL_BUILT: AtomicBool = AtomicBool::new(false);
static UEFI_BUILT: AtomicBool = AtomicBool::new(false);

fn build_kernel() -> XtaskResult<()> {
    if KERNEL_BUILT.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    run_command(
        "cargo",
        &[
            "build",
            "-p",
            "txpos-kernel",
            "--bin",
            "txpos-kernel-entry",
            "--features",
            "kernel-bin",
            "--release",
            "--target",
            "x86_64-unknown-none",
        ],
    )
}

fn build_uefi() -> XtaskResult<()> {
    if UEFI_BUILT.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    run_command(
        "cargo",
        &[
            "build",
            "-p",
            "txpos-bootloader",
            "--bin",
            "txpos-bootloader-uefi",
            "--features",
            "uefi-bin",
            "--release",
            "--target",
            "x86_64-unknown-uefi",
        ],
    )
}

fn stage_esp() -> XtaskResult<()> {
    build_uefi()?;
    build_kernel()?;

    let dist = Path::new(DIST_DIR);
    let esp = Path::new(ESP_DIR);
    let boot_dir = esp.join("EFI").join("BOOT");
    let txpos_dir = esp.join("TXPOS");

    fs::create_dir_all(&boot_dir)?;
    fs::create_dir_all(&txpos_dir)?;

    let bootloader = Path::new("target")
        .join("x86_64-unknown-uefi")
        .join("release")
        .join("txpos-bootloader-uefi.efi");
    let kernel = Path::new("target")
        .join("x86_64-unknown-none")
        .join("release")
        .join("txpos-kernel-entry");

    require_file(
        &bootloader,
        "UEFI bootloader is missing. Run: cargo run -p xtask -- build-uefi",
    )?;
    require_file(
        &kernel,
        "kernel image is missing. Run: cargo run -p xtask -- build-kernel",
    )?;

    let staged_bootloader = boot_dir.join("BOOTX64.EFI");
    let staged_kernel = txpos_dir.join("KERNEL.BIN");
    fs::copy(&bootloader, &staged_bootloader)?;
    fs::copy(&bootloader, dist.join("BOOTX64.EFI"))?;
    fs::copy(&kernel, &staged_kernel)?;

    let digest = sha256_hex(&fs::read(&staged_kernel)?);
    let manifest = format!(
        "version=1\r\nkernel_path=\\\\TXPOS\\\\KERNEL.BIN\r\nkernel_sha256={digest}\r\nboot_policy=development\r\n"
    );
    fs::write(txpos_dir.join("MANIFEST.TXT"), manifest)?;

    println!("staged EFI system partition at dist/esp");
    Ok(())
}

fn build_img() -> XtaskResult<()> {
    stage_esp()?;

    let payload = EspPayload::load()?;
    let volume = create_fat32_volume(&payload, 0);
    fs::write(Path::new(DIST_DIR).join("esp.img"), &volume)?;

    let mut disk = vec![0u8; PARTITION_START_LBA as usize * BYTES_PER_SECTOR + volume.len()];
    write_mbr(&mut disk, PARTITION_START_LBA, FAT_VOLUME_SECTORS);
    let offset = PARTITION_START_LBA as usize * BYTES_PER_SECTOR;
    disk[offset..offset + volume.len()].copy_from_slice(&volume);
    fs::write(Path::new(DIST_DIR).join("txpos.img"), disk)?;

    println!("created dist/txpos.img");
    Ok(())
}

fn build_iso() -> XtaskResult<()> {
    stage_esp()?;

    let payload = EspPayload::load()?;
    let esp_image = create_fat32_volume(&payload, 0);
    let iso = create_uefi_iso(&esp_image);
    fs::write(Path::new(DIST_DIR).join("txpos.iso"), iso)?;

    println!("created dist/txpos.iso");
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> XtaskResult<()> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(format!("{program} {} failed with status {status}", args.join(" ")).into());
    }

    Ok(())
}

fn require_file(path: &Path, message: &str) -> XtaskResult<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{message}\nmissing file: {}", path.display()).into())
    }
}

struct EspPayload {
    bootloader: Vec<u8>,
    kernel: Vec<u8>,
    manifest: Vec<u8>,
}

impl EspPayload {
    fn load() -> XtaskResult<Self> {
        Ok(Self {
            bootloader: fs::read(
                Path::new(ESP_DIR)
                    .join("EFI")
                    .join("BOOT")
                    .join("BOOTX64.EFI"),
            )?,
            kernel: fs::read(Path::new(ESP_DIR).join("TXPOS").join("KERNEL.BIN"))?,
            manifest: fs::read(Path::new(ESP_DIR).join("TXPOS").join("MANIFEST.TXT"))?,
        })
    }
}

fn create_fat32_volume(payload: &EspPayload, hidden_sectors: u32) -> Vec<u8> {
    let fat_size = calculate_fat_size(FAT_VOLUME_SECTORS);
    let first_data_sector = RESERVED_SECTORS + FAT_COUNT * fat_size;
    let cluster_count = (FAT_VOLUME_SECTORS - first_data_sector) / SECTORS_PER_CLUSTER;
    let cluster_size = SECTORS_PER_CLUSTER as usize * BYTES_PER_SECTOR;
    let mut volume = vec![0u8; FAT_VOLUME_SECTORS as usize * BYTES_PER_SECTOR];
    let mut fat = vec![0u32; cluster_count as usize + 2];
    let mut next_cluster = 6u32;

    fat[0] = 0x0fff_fff8;
    fat[1] = 0xffff_ffff;
    for cluster in 2..=5 {
        fat[cluster as usize] = 0x0fff_ffff;
    }

    let bootloader_chain = allocate_chain(
        &mut fat,
        &mut next_cluster,
        payload.bootloader.len(),
        cluster_size,
    );
    let kernel_chain = allocate_chain(
        &mut fat,
        &mut next_cluster,
        payload.kernel.len(),
        cluster_size,
    );
    let manifest_chain = allocate_chain(
        &mut fat,
        &mut next_cluster,
        payload.manifest.len(),
        cluster_size,
    );

    write_fat32_boot_sector(&mut volume, fat_size, hidden_sectors);
    write_fs_info_sector(&mut volume);
    let backup_offset = 6 * BYTES_PER_SECTOR;
    let boot_sector = volume[..BYTES_PER_SECTOR].to_vec();
    volume[backup_offset..backup_offset + BYTES_PER_SECTOR].copy_from_slice(&boot_sector);
    write_fats(&mut volume, &fat, fat_size);

    write_root_directory(&mut volume, first_data_sector);
    write_efi_directory(&mut volume, first_data_sector);
    write_boot_directory(
        &mut volume,
        first_data_sector,
        &bootloader_chain,
        payload.bootloader.len(),
    );
    write_txpos_directory(
        &mut volume,
        first_data_sector,
        &kernel_chain,
        payload.kernel.len(),
        &manifest_chain,
        payload.manifest.len(),
    );
    write_file_chain(
        &mut volume,
        first_data_sector,
        &bootloader_chain,
        &payload.bootloader,
    );
    write_file_chain(
        &mut volume,
        first_data_sector,
        &kernel_chain,
        &payload.kernel,
    );
    write_file_chain(
        &mut volume,
        first_data_sector,
        &manifest_chain,
        &payload.manifest,
    );

    volume
}

fn calculate_fat_size(total_sectors: u32) -> u32 {
    let mut fat_size = 1;
    loop {
        let data_sectors = total_sectors - RESERVED_SECTORS - FAT_COUNT * fat_size;
        let clusters = data_sectors / SECTORS_PER_CLUSTER;
        let new_fat_size = ((clusters + 2) * 4).div_ceil(BYTES_PER_SECTOR as u32);
        if new_fat_size <= fat_size {
            return fat_size;
        }
        fat_size = new_fat_size;
    }
}

fn allocate_chain(
    fat: &mut [u32],
    next_cluster: &mut u32,
    file_len: usize,
    cluster_size: usize,
) -> Vec<u32> {
    let count = file_len.max(1).div_ceil(cluster_size);
    let start = *next_cluster;
    let mut chain = Vec::with_capacity(count);

    for offset in 0..count {
        let cluster = start + offset as u32;
        chain.push(cluster);
        fat[cluster as usize] = if offset + 1 == count {
            0x0fff_ffff
        } else {
            cluster + 1
        };
    }

    *next_cluster += count as u32;
    chain
}

fn write_mbr(image: &mut [u8], partition_start: u32, partition_sectors: u32) {
    let entry = 446;
    image[entry + 4] = 0xef;
    image[entry + 8..entry + 12].copy_from_slice(&partition_start.to_le_bytes());
    image[entry + 12..entry + 16].copy_from_slice(&partition_sectors.to_le_bytes());
    image[510] = 0x55;
    image[511] = 0xaa;
}

fn write_fat32_boot_sector(volume: &mut [u8], fat_size: u32, hidden_sectors: u32) {
    let sector = &mut volume[..BYTES_PER_SECTOR];
    sector[0..3].copy_from_slice(&[0xeb, 0x58, 0x90]);
    sector[3..11].copy_from_slice(b"TXPOS   ");
    sector[11..13].copy_from_slice(&(BYTES_PER_SECTOR as u16).to_le_bytes());
    sector[13] = SECTORS_PER_CLUSTER as u8;
    sector[14..16].copy_from_slice(&(RESERVED_SECTORS as u16).to_le_bytes());
    sector[16] = FAT_COUNT as u8;
    sector[21] = 0xf8;
    sector[24..26].copy_from_slice(&63u16.to_le_bytes());
    sector[26..28].copy_from_slice(&255u16.to_le_bytes());
    sector[28..32].copy_from_slice(&hidden_sectors.to_le_bytes());
    sector[32..36].copy_from_slice(&FAT_VOLUME_SECTORS.to_le_bytes());
    sector[36..40].copy_from_slice(&fat_size.to_le_bytes());
    sector[44..48].copy_from_slice(&2u32.to_le_bytes());
    sector[48..50].copy_from_slice(&1u16.to_le_bytes());
    sector[50..52].copy_from_slice(&6u16.to_le_bytes());
    sector[64] = 0x80;
    sector[66] = 0x29;
    sector[67..71].copy_from_slice(&0x5458_504f_u32.to_le_bytes());
    sector[71..82].copy_from_slice(b"TXPOS      ");
    sector[82..90].copy_from_slice(b"FAT32   ");
    sector[510] = 0x55;
    sector[511] = 0xaa;
}

fn write_fs_info_sector(volume: &mut [u8]) {
    let start = BYTES_PER_SECTOR;
    let sector = &mut volume[start..start + BYTES_PER_SECTOR];
    sector[0..4].copy_from_slice(&0x4161_5252_u32.to_le_bytes());
    sector[484..488].copy_from_slice(&0x6141_7272_u32.to_le_bytes());
    sector[488..492].copy_from_slice(&0xffff_ffff_u32.to_le_bytes());
    sector[492..496].copy_from_slice(&0xffff_ffff_u32.to_le_bytes());
    sector[508..512].copy_from_slice(&0xaa55_0000_u32.to_le_bytes());
}

fn write_fats(volume: &mut [u8], fat: &[u32], fat_size: u32) {
    let fat_bytes = fat_size as usize * BYTES_PER_SECTOR;
    for fat_index in 0..FAT_COUNT as usize {
        let start = (RESERVED_SECTORS as usize * BYTES_PER_SECTOR) + fat_index * fat_bytes;
        let table = &mut volume[start..start + fat_bytes];
        for (entry_index, entry) in fat.iter().enumerate() {
            let offset = entry_index * 4;
            if offset + 4 <= table.len() {
                table[offset..offset + 4].copy_from_slice(&entry.to_le_bytes());
            }
        }
    }
}

fn write_root_directory(volume: &mut [u8], first_data_sector: u32) {
    let root = cluster_mut(volume, first_data_sector, 2);
    write_dir_entry(&mut root[0..32], b"EFI        ", 0x10, 3, 0);
    write_dir_entry(&mut root[32..64], b"TXPOS      ", 0x10, 5, 0);
}

fn write_efi_directory(volume: &mut [u8], first_data_sector: u32) {
    let dir = cluster_mut(volume, first_data_sector, 3);
    write_dir_entry(&mut dir[0..32], b".          ", 0x10, 3, 0);
    write_dir_entry(&mut dir[32..64], b"..         ", 0x10, 2, 0);
    write_dir_entry(&mut dir[64..96], b"BOOT       ", 0x10, 4, 0);
}

fn write_boot_directory(
    volume: &mut [u8],
    first_data_sector: u32,
    bootloader_chain: &[u32],
    bootloader_len: usize,
) {
    let dir = cluster_mut(volume, first_data_sector, 4);
    write_dir_entry(&mut dir[0..32], b".          ", 0x10, 4, 0);
    write_dir_entry(&mut dir[32..64], b"..         ", 0x10, 3, 0);
    write_dir_entry(
        &mut dir[64..96],
        b"BOOTX64 EFI",
        0x20,
        bootloader_chain[0],
        bootloader_len as u32,
    );
}

fn write_txpos_directory(
    volume: &mut [u8],
    first_data_sector: u32,
    kernel_chain: &[u32],
    kernel_len: usize,
    manifest_chain: &[u32],
    manifest_len: usize,
) {
    let dir = cluster_mut(volume, first_data_sector, 5);
    write_dir_entry(&mut dir[0..32], b".          ", 0x10, 5, 0);
    write_dir_entry(&mut dir[32..64], b"..         ", 0x10, 2, 0);
    write_dir_entry(
        &mut dir[64..96],
        b"KERNEL  BIN",
        0x20,
        kernel_chain[0],
        kernel_len as u32,
    );
    write_dir_entry(
        &mut dir[96..128],
        b"MANIFESTTXT",
        0x20,
        manifest_chain[0],
        manifest_len as u32,
    );
}

fn write_dir_entry(entry: &mut [u8], name: &[u8; 11], attrs: u8, cluster: u32, size: u32) {
    entry.fill(0);
    entry[0..11].copy_from_slice(name);
    entry[11] = attrs;
    entry[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    entry[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
}

fn write_file_chain(volume: &mut [u8], first_data_sector: u32, chain: &[u32], data: &[u8]) {
    let cluster_size = SECTORS_PER_CLUSTER as usize * BYTES_PER_SECTOR;
    for (index, cluster) in chain.iter().enumerate() {
        let start = index * cluster_size;
        let end = data.len().min(start + cluster_size);
        if start >= end {
            break;
        }
        let target = cluster_mut(volume, first_data_sector, *cluster);
        target[..end - start].copy_from_slice(&data[start..end]);
    }
}

fn cluster_mut(volume: &mut [u8], first_data_sector: u32, cluster: u32) -> &mut [u8] {
    let sector = first_data_sector + (cluster - 2) * SECTORS_PER_CLUSTER;
    let offset = sector as usize * BYTES_PER_SECTOR;
    let len = SECTORS_PER_CLUSTER as usize * BYTES_PER_SECTOR;
    &mut volume[offset..offset + len]
}

fn create_uefi_iso(esp_image: &[u8]) -> Vec<u8> {
    let esp_sector_count = esp_image.len().div_ceil(ISO_SECTOR_SIZE);
    let path_table_lba = 19u32;
    let root_dir_lba = 20u32;
    let boot_catalog_lba = 21u32;
    let esp_lba = 22u32;
    let total_sectors = esp_lba as usize + esp_sector_count;
    let mut iso = vec![0u8; total_sectors * ISO_SECTOR_SIZE];

    write_primary_volume_descriptor(
        sector_mut(&mut iso, 16),
        total_sectors as u32,
        path_table_lba,
        root_dir_lba,
    );
    write_boot_record_descriptor(sector_mut(&mut iso, 17), boot_catalog_lba);
    write_volume_terminator(sector_mut(&mut iso, 18));
    write_path_table(sector_mut(&mut iso, path_table_lba as usize), root_dir_lba);
    write_iso_root_directory(
        sector_mut(&mut iso, root_dir_lba as usize),
        root_dir_lba,
        esp_lba,
        esp_image.len() as u32,
    );
    write_boot_catalog(sector_mut(&mut iso, boot_catalog_lba as usize), esp_lba);

    let esp_offset = esp_lba as usize * ISO_SECTOR_SIZE;
    iso[esp_offset..esp_offset + esp_image.len()].copy_from_slice(esp_image);
    iso
}

fn sector_mut(image: &mut [u8], sector: usize) -> &mut [u8] {
    let offset = sector * ISO_SECTOR_SIZE;
    &mut image[offset..offset + ISO_SECTOR_SIZE]
}

fn write_primary_volume_descriptor(
    sector: &mut [u8],
    total_sectors: u32,
    path_table_lba: u32,
    root_dir_lba: u32,
) {
    sector[0] = 1;
    sector[1..6].copy_from_slice(b"CD001");
    sector[6] = 1;
    write_padded_ascii(&mut sector[8..40], b"TXPOS");
    write_padded_ascii(&mut sector[40..72], b"TXPOS_BOOT");
    write_both_endian_u32(&mut sector[80..88], total_sectors);
    write_both_endian_u16(&mut sector[120..124], 1);
    write_both_endian_u16(&mut sector[124..128], 1);
    write_both_endian_u16(&mut sector[128..132], ISO_SECTOR_SIZE as u16);
    write_both_endian_u32(&mut sector[132..140], 10);
    sector[140..144].copy_from_slice(&path_table_lba.to_le_bytes());
    sector[148..152].copy_from_slice(&path_table_lba.to_be_bytes());
    write_iso_dir_record(
        &mut sector[156..190],
        root_dir_lba,
        ISO_SECTOR_SIZE as u32,
        0x02,
        &[0],
    );
    write_padded_ascii(&mut sector[318..446], b"TXPOS");
    write_padded_ascii(&mut sector[446..574], b"TXPOS");
    write_padded_ascii(&mut sector[574..702], b"TXPOS");
    write_padded_ascii(&mut sector[702..830], b"TXPOS");
    write_iso_datetime(&mut sector[813..830]);
}

fn write_boot_record_descriptor(sector: &mut [u8], boot_catalog_lba: u32) {
    sector[0] = 0;
    sector[1..6].copy_from_slice(b"CD001");
    sector[6] = 1;
    write_padded_ascii(&mut sector[7..39], b"EL TORITO SPECIFICATION");
    sector[71..75].copy_from_slice(&boot_catalog_lba.to_le_bytes());
}

fn write_volume_terminator(sector: &mut [u8]) {
    sector[0] = 255;
    sector[1..6].copy_from_slice(b"CD001");
    sector[6] = 1;
}

fn write_path_table(sector: &mut [u8], root_dir_lba: u32) {
    sector[0] = 1;
    sector[1] = 0;
    sector[2..6].copy_from_slice(&root_dir_lba.to_le_bytes());
    sector[6..8].copy_from_slice(&1u16.to_le_bytes());
    sector[8] = 0;
}

fn write_iso_root_directory(sector: &mut [u8], root_dir_lba: u32, esp_lba: u32, esp_len: u32) {
    let mut offset = 0;
    offset += write_iso_dir_record(
        &mut sector[offset..],
        root_dir_lba,
        ISO_SECTOR_SIZE as u32,
        0x02,
        &[0],
    );
    offset += write_iso_dir_record(
        &mut sector[offset..],
        root_dir_lba,
        ISO_SECTOR_SIZE as u32,
        0x02,
        &[1],
    );
    let _ = write_iso_dir_record(
        &mut sector[offset..],
        esp_lba,
        esp_len,
        0x00,
        b"TXPOS.IMG;1",
    );
}

fn write_boot_catalog(sector: &mut [u8], esp_lba: u32) {
    sector[0] = 1;
    sector[1] = 0xef;
    write_padded_ascii(&mut sector[4..28], b"TXPOS");
    sector[30] = 0x55;
    sector[31] = 0xaa;

    let checksum = boot_catalog_checksum(&sector[0..32]);
    sector[28..30].copy_from_slice(&checksum.to_le_bytes());

    let entry = &mut sector[32..64];
    entry[0] = 0x88;
    entry[1] = 0;
    entry[6..8].copy_from_slice(&1u16.to_le_bytes());
    entry[8..12].copy_from_slice(&esp_lba.to_le_bytes());
}

fn boot_catalog_checksum(entry: &[u8]) -> u16 {
    let mut sum = 0u32;
    for index in (0..32).step_by(2) {
        if index == 28 {
            continue;
        }
        sum = sum.wrapping_add(u16::from_le_bytes([entry[index], entry[index + 1]]) as u32);
    }
    (!sum as u16).wrapping_add(1)
}

fn write_iso_dir_record(
    target: &mut [u8],
    extent_lba: u32,
    data_len: u32,
    flags: u8,
    file_id: &[u8],
) -> usize {
    let padding = if file_id.len() % 2 == 0 { 1 } else { 0 };
    let len = 33 + file_id.len() + padding;
    target[..len].fill(0);
    target[0] = len as u8;
    write_both_endian_u32(&mut target[2..10], extent_lba);
    write_both_endian_u32(&mut target[10..18], data_len);
    write_iso_short_datetime(&mut target[18..25]);
    target[25] = flags;
    write_both_endian_u16(&mut target[28..32], 1);
    target[32] = file_id.len() as u8;
    target[33..33 + file_id.len()].copy_from_slice(file_id);
    len
}

fn write_both_endian_u16(target: &mut [u8], value: u16) {
    target[0..2].copy_from_slice(&value.to_le_bytes());
    target[2..4].copy_from_slice(&value.to_be_bytes());
}

fn write_both_endian_u32(target: &mut [u8], value: u32) {
    target[0..4].copy_from_slice(&value.to_le_bytes());
    target[4..8].copy_from_slice(&value.to_be_bytes());
}

fn write_padded_ascii(target: &mut [u8], text: &[u8]) {
    target.fill(b' ');
    let len = target.len().min(text.len());
    target[..len].copy_from_slice(&text[..len]);
}

fn write_iso_datetime(target: &mut [u8]) {
    target.copy_from_slice(b"2026070100000000\0");
}

fn write_iso_short_datetime(target: &mut [u8]) {
    target.copy_from_slice(&[126, 7, 1, 0, 0, 0, 0]);
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(hex_digit(byte >> 4));
        output.push(hex_digit(byte & 0x0f));
    }
    output
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => '?',
    }
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let bit_len = (input.len() as u64) * 8;
    let padded_len = ((input.len() + 9).div_ceil(64)) * 64;
    let mut padded = vec![0u8; padded_len];
    padded[..input.len()].copy_from_slice(input);
    padded[input.len()] = 0x80;
    padded[padded_len - 8..].copy_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }

        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut digest = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fat_size_is_stable_for_release_volume() {
        assert_eq!(calculate_fat_size(FAT_VOLUME_SECTORS), 1024);
    }

    #[test]
    fn boot_catalog_checksum_balances_validation_entry() {
        let mut sector = [0u8; ISO_SECTOR_SIZE];
        write_boot_catalog(&mut sector, 22);
        let sum = sector[..32]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]) as u32)
            .sum::<u32>();
        assert_eq!(sum & 0xffff, 0);
    }
}
