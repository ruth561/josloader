#!/bin/bash

# EFIアプリケーションとしてプログラムをビルドする。
cargo build --target x86_64-unknown-uefi
# qemuで使うファイルシステムはfs/以下に作成する。
# 作成したEFIブートローダーは、/EFI/BOOT/BOOTX64.EFIという場所に
# 配置しておくことで、UEFIからブートローダーとして認識されるらしい。
mkdir -p fs/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/josloader.efi fs/EFI/BOOT/BOOTX64.EFI
# ファイルシステムのrootディレクトリにカーネルイメージを置いておくと、
# そのカーネルイメージがメモリ0x100000にロードされ、0x100000から実行が始まる。
# 今回は0x100000で無限ループするような命令を先頭に配置する。
# eb fe とは以下のような命令。
# loop:
#       jmp loop
echo -e '\xeb\xfe' > fs/kernel.elf

# デフォルトではQEMUにUEFIは入っていないので、別途用意してあげる必要がある。
# ovmfが便利なので、もしシステムにinstallされていなかったら、ovmfを
# インストールするようにした。
if [ ! -f /usr/share/ovmf/OVMF.fd ]; then
        echo "[+] You should install ovmf!"
        sudo apt install ovmf
fi

qemu-system-x86_64 -m 4G \
	-bios /usr/share/ovmf/OVMF.fd \
	-drive file=fat:rw:fs,media=disk,format=raw \
	-monitor stdio -s

