[BITS 16]
[ORG 0x8000]             ; SIPI vector (start page * 0x1000)

start16:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00        ; temporary stack

    call enable_a20

    ; Load GDT for 32-bit transition
    lgdt [gdt_ptr]

    ; Enter 32-bit protected mode
    mov eax, cr0
    or eax, 1             ; Set PE bit
    mov cr0, eax
    jmp CODE32_SEL:pm32   ; Far jump to flush pipeline

[BITS 32]
pm32:
    ; Update segments
    mov ax, DATA32_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Load GDT for 64-bit mode (same or updated)
    lgdt [gdt_ptr]

    ; Enable PAE and LME
    mov eax, cr4
    or eax, 1 << 5         ; Set PAE
    mov cr4, eax

    ; Load page table from trampoline data
    mov eax, [TRAMPOLINE_DATA_PTR + 0x10]  ; offset of page_table
    mov cr3, eax

    ; Enable long mode
    mov ecx, 0xC0000080     ; EFER MSR
    rdmsr
    or eax, 1 << 8          ; LME bit
    wrmsr

    ; Enable paging
    mov eax, cr0
    or eax, 1 << 31         ; PG bit
    mov cr0, eax

    ; Far jump to long mode
    jmp CODE64_SEL:long_mode

[BITS 64]
long_mode:
    ; Load stack pointer
    mov rsp, qword [TRAMPOLINE_DATA_PTR + 0x00]

    ; Call AP entry function
    mov rax, qword [TRAMPOLINE_DATA_PTR + 0x08]
    call rax

    hlt
    jmp $

; -------------------------------------------------------------------
; Data section

[SECTION .data]

align 8
gdt64:
    dq 0x0000000000000000         ; null descriptor
    dq 0x00af9a000000ffff         ; 64-bit code
    dq 0x00af92000000ffff         ; 64-bit data

gdt_ptr:
    dw gdt64_end - gdt64 - 1
    dd gdt64

gdt64_end:

TRAMPOLINE_DATA_PTR equ 0x9000

; -------------------------------------------------------------------
; A20 Line Enabler (simplified)

enable_a20:
    in al, 0x92
    or al, 2
    out 0x92, al
    ret

; Segment selectors
CODE32_SEL equ 0x08
DATA32_SEL equ 0x10
CODE64_SEL equ 0x08
