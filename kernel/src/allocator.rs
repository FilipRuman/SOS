mod FixedSize;

use alloc::{
    alloc::{GlobalAlloc, Layout},
    borrow::ToOwned,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use log::debug;

use core::ptr::null_mut;

pub struct Locked<T> {
    inner: spin::Mutex<T>,
}
impl<T> Locked<T> {
    pub const fn new(inner: T) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<T> {
        self.inner.lock()
    }
}

#[global_allocator]
static ALLOCATOR: Locked<BuddyAllocator> = Locked::new(BuddyAllocator::new());

pub struct Dummy;
unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB, mapper::MapToError,
    },
};

use crate::{
    allocator::FixedSize::BuddyAllocator,
    memory::{self, FRAMES, HEAP_SIZE, HEAP_START, MAPPER, StaticFrameAllocator},
};
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    let mut frame_allocator = StaticFrameAllocator {};
    let mut mapper = MAPPER.get().expect("Memory was not yet initialized").lock();

    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)?
                .flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    log::debug!("allocator was initialized- you can use alloc functions from now on!");

    // allocator_tests();
    Ok(())
}

pub fn allocator_tests() {
    // debug!("free list {:#?}", ALLOCATOR.lock().free_list);
    let mut init_large_bits = 1;

    debug!(
        "============================================== init ====================================="
    );
    {
        for _ in 0..5 {
            let x = Box::new(52usize);

            let w = Box::new(52usize);

            let d = Box::new(52usize);

            let s = Box::new(52usize);

            let l = Box::new(52usize);

            let k = Box::new(52usize);

            let j = Box::new(52usize);
        }

        debug!(
            "============================================== alloc ====================================="
        );
        //debug!("free list {:#?}", ALLOCATOR.lock().free_list);
    }

    debug!(
        "============================================== dealoc ====================================="
    );

    debug!("free list {:#?}", ALLOCATOR.lock().free_list);

    debug!("free list {:#?}", ALLOCATOR.lock().free_list);

    // debug!("free list {:#?}", ALLOCATOR.lock().free_list);

    // for _ in 0..100 {
    // {
    //     log::debug!("started allocator tests!");
    //     let string: String = "test_string".to_string();
    //
    //     let mut test_struct_vec = Vec::new();
    //     for _ in 0..10 {
    //         test_struct_vec.push(ReallyBigTestStruct::new());
    //     }
    //     debug!("tst:{}", test_struct_vec.len());
    //     debug!("string: {string}");
    //     assert_eq!(string.as_str(), "test_string");
    // }
}

pub struct ReallyBigTestStruct {
    pub array: [usize; 20],
    pub vec: Vec<usize>,
    pub test: String,

    pub array3: [usize; 30],
    pub array2: [usize; 100],
    pub test_1: String,
}
impl ReallyBigTestStruct {
    pub fn new() -> ReallyBigTestStruct {
        ReallyBigTestStruct {
            array: [0; 20],
            array2: [20; 100],
            array3: [3; 30],
            test_1: "IWERIOTIEROITROITEWOI:WRETIWEOROTIUERIOTIUERUWRTUIOREWTUREHGJKDHJdkskljfdgglshgjkdkjldgflgflhkgdslhkjfgdsjsdffsdlgkhfghjfkdsgfhjdkldfsgklhjkgdfshjklfdghkhjkgfdslkdgkhgflhjskdfglskhljksdgfhkjsdfgslkhdfhdlkjfghjkglfhklhskdfghjsjhlk".to_string(),
            vec: [20; 100].to_vec(),
            test: "IWERIOTIEROITROITEWOI:WRETIWEOROTIUERIOTIUERUWRTUIOREWTUREHGJKDHJdkskljfdgglshgjkdkjldgflgflhkgdslhkjfgdsjsdffsdlgkhfghjfkdsgfhjdkldfsgklhjkgdfshjklfdghkhjkgfdslkdgkhgflhjskdfglskhljksdgfhkjsdfgslkhdfhdlkjfghjkglfhklhskdfghjsjhlk".to_string(),
        }
    }
}
