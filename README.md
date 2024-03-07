# josloader

This is the bootloader for jos.

If you want your operating system to be booted by this bootloader, you need to link it with the assumption that it will be loaded at memory address 0x100000. By preparing a file system like the following and placing your OS in kernel.elf, your OS can also be booted.

```
.
├── EFI
│   └── BOOT
│       └── BOOTX64.EFI    <---- josloader
│
└── kernel.elf              <---- your kernel
```
