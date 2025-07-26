use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

use crate::allocator::Locked;

#[repr(C)]
pub struct AllocatorNode {
    pub start_addr: usize, // start address of the memory block that it represents (also start address of this node)
    //WARN: always has to be a power of two
    pub size: usize, // the size of memory block that this represents
    pub free_list_index: usize,
    pub next_in_free_list: Option<&'static mut AllocatorNode>,
}
impl core::fmt::Debug for AllocatorNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Node {{ addr{} idx{} \n {:?} ",
            self.start_addr, self.free_list_index, self.next_in_free_list
        )
    }
}

impl AllocatorNode {
    // just creates 2 new nodes
    // you have to setup free list settings later
    unsafe fn split(&'static mut self) -> (&mut AllocatorNode, &mut AllocatorNode) {
        let new_size = self.size / 2;

        let r_start_addr = self.start_addr + new_size;
        let r_pointer = (r_start_addr) as *mut AllocatorNode;

        unsafe {
            r_pointer.write(AllocatorNode {
                start_addr: r_start_addr,
                free_list_index: self.free_list_index - 1,
                next_in_free_list: None,
                size: new_size,
            })
        };

        check_pointer_safety(r_pointer, "r_pointer");

        let l_pointer = self.start_addr as *mut AllocatorNode;
        // this will override this node, so be careful!

        unsafe {
            l_pointer.write(AllocatorNode {
                start_addr: self.start_addr,
                free_list_index: self.free_list_index - 1,
                next_in_free_list: None,
                size: new_size,
            })
        };
        check_pointer_safety(l_pointer, "l_pointer");

        // dereferences the raw pointer to access the value stored at that memory location.
        // then take a mutable reference to that dereferenced value
        unsafe { (&mut *l_pointer, &mut *r_pointer) }
    }
}

fn check_pointer_safety(pointer: *mut AllocatorNode, pointer_name: &str) {
    assert_eq!(
        (pointer as usize) % core::mem::align_of::<AllocatorNode>(),
        0,
        "dealloc_ptr is misaligned for AllocatorNode, with name: {pointer_name}"
    );
    assert!(!pointer.is_null(), "dealloc_ptr is null");
    assert_eq!(
        pointer as usize,
        unsafe { (*pointer).start_addr },
        "AllocatorNode start address != ptr address, with name: {pointer_name}"
    )
}

pub fn free_list_index(size: usize) -> usize {
    let order = size.ilog2() as usize;

    if order >= MAX_ORDER_EXCLUSIVE {
        panic!(
            "max order in fixed size allocator was too small: size {size} order {order} max order {MAX_ORDER_EXCLUSIVE}"
        )
    }
    order - MIN_ORDER_INCLUSIVE
}
const LIST_SIZE: usize = MAX_ORDER_EXCLUSIVE - MIN_ORDER_INCLUSIVE;
const MAX_ORDER_EXCLUSIVE: usize = 19; // Max block size = 
const MIN_ORDER_INCLUSIVE: usize = 5; //  Min block sized = 32
const MIN_SIZE: usize = 2usize.pow(MIN_ORDER_INCLUSIVE as u32);

pub struct BuddyAllocator {
    pub free_list: [Option<&'static mut AllocatorNode>; LIST_SIZE],
}
impl BuddyAllocator {
    pub const fn new() -> BuddyAllocator {
        const EMPTY: Option<&'static mut AllocatorNode> = None;
        BuddyAllocator {
            free_list: [EMPTY; LIST_SIZE],
        }
    }
    pub unsafe fn init(&mut self, heap_start_addr: usize, heap_size: usize) {
        // create the biggest nodes for the allocator pool
        // -1 because exclusive
        let nodes_size = 2usize.pow(MAX_ORDER_EXCLUSIVE as u32 - 1);

        let mut current_node = unsafe {
            let init_node = AllocatorNode {
                start_addr: heap_start_addr,
                size: nodes_size,
                free_list_index: LIST_SIZE - 1,
                next_in_free_list: None,
            };

            let init_pointer = heap_start_addr as *mut AllocatorNode;
            (init_pointer).write(init_node);
            &mut *init_pointer
        };

        let nodes_count = heap_size / nodes_size;

        for i in 1..nodes_count {
            let addr = heap_start_addr + i * nodes_size;

            let node_heap = unsafe {
                let node_stack = AllocatorNode {
                    start_addr: addr,
                    size: nodes_size,
                    free_list_index: LIST_SIZE - 1,
                    next_in_free_list: Some(current_node),
                };
                let pointer = addr as *mut AllocatorNode;
                (pointer).write(node_stack);
                &mut *pointer
            };

            current_node = node_heap;
        }

        self.free_list[LIST_SIZE - 1] = Some(current_node);

        log::debug!("initialized buddy allocator");
    }

    fn remove_from_free_list(&mut self, free_list_index: usize, node: &'static mut AllocatorNode) {
        match self.free_list[free_list_index].take() {
            Some(head) => node.next_in_free_list = Some(head),
            None => {}
        };

        self.free_list[free_list_index] = Some(node);
    }
    fn add_to_free_list(&mut self, free_list_index: usize, node: &'static mut AllocatorNode) {
        match self.free_list[free_list_index].take() {
            Some(head) => node.next_in_free_list = Some(head),
            None => {}
        };

        self.free_list[free_list_index] = Some(node);
    }

    // returns:  buddy and it's partent on free list (in option because head of list doesn't have a
    // parent!)
    pub fn get_free_buddy_and_its_parent(
        &mut self,
        node: &AllocatorNode,
    ) -> Option<(
        &'static mut AllocatorNode,         //buddy
        Option<&'static mut AllocatorNode>, // parent of buddy
    )> {
        let buddy_start_address = node.start_addr ^ node.size;
        let mut previous_addr: Option<usize> = None;
        let mut current_option = &mut self.free_list[node.free_list_index];

        while let Some(node_to_check) = current_option {
            if node_to_check.start_addr == buddy_start_address {
                let buddy = unsafe { &mut *(buddy_start_address as *mut AllocatorNode) };
                match previous_addr {
                    Some(addr) => {
                        let parent = unsafe { &mut *(addr as *mut AllocatorNode) };
                        return Some((buddy, Some(parent)));
                    }
                    None => {
                        return Some((buddy, None));
                    }
                }
            }

            previous_addr = Some(node_to_check.start_addr);
            current_option = &mut node_to_check.next_in_free_list;
        }

        None
    }

    // splits existing node  and returns left child
    // WARN: skips the requested size on free list, it assumes you have already checked and it was NONE
    // old node is removed from free list
    //
    // new, right child is on free list
    // none - out of memory
    unsafe fn get_new_node_of_size(
        &mut self,
        requested_size_list_index: usize,
    ) -> Option<&'static mut AllocatorNode> {
        // assume that requested size was already checked
        let mut current_size_list_index = requested_size_list_index + 1;
        while current_size_list_index < LIST_SIZE {
            match self.free_list[current_size_list_index].take() {
                Some(node_to_split) => {
                    // make the next node a head
                    self.free_list[current_size_list_index] =
                        node_to_split.next_in_free_list.take();

                    // debug!(
                    //     "run split_node_i_times_and_return_last_node: {:?} {} {} {:?}",
                    //     current_size_list_index,
                    //     requested_size_list_index,
                    //     current_size_list_index - requested_size_list_index,
                    //     node_to_split,
                    // );
                    // there is a node that you can split
                    return Some(unsafe {
                        self.split_node_i_times_and_return_last_node(
                            current_size_list_index - requested_size_list_index,
                            node_to_split,
                        )
                    });
                }
                None => {
                    current_size_list_index += 1;
                }
            }
        }
        panic!("fixed size allocator is out of memory!");
        // out of memory
        None
    }

    // all nodes other then the returned one will be added to free list
    unsafe fn split_node_i_times_and_return_last_node(
        &mut self,
        i: usize,
        target_node: &'static mut AllocatorNode,
    ) -> &'static mut AllocatorNode {
        let mut current_free_list_index = target_node.free_list_index;
        let mut current_node = target_node;
        for _ in 0..i {
            // the node should be assigned -1 after split so:
            current_free_list_index -= 1;

            let (l_node, r_node) = unsafe { current_node.split() };

            self.add_to_free_list(current_free_list_index, r_node);
            // you don't add left node because it will be further split / returned and allocated
            current_node = l_node;
        }
        current_node
    }
    // maybe change this to while loop
    unsafe fn recursively_dealloc_and_connect_buddy_nodes(
        &mut self,
        node: AllocatorNode,
        dealloc_ptr: *mut u8,
    ) {
        if node.free_list_index < LIST_SIZE - 1 {
            match self.get_free_buddy_and_its_parent(&node) {
                Some((buddy, parent_of_buddy_option)) => {
                    // debug!("remove buddy: {:?}", &buddy);
                    // remove buddy from free list
                    if let Some(parent_of_buddy) = parent_of_buddy_option {
                        parent_of_buddy.next_in_free_list = buddy.next_in_free_list.take();
                    } else {
                        self.free_list[buddy.free_list_index] = buddy.next_in_free_list.take();
                    }

                    // take one from left node
                    let start_addr = node.start_addr.min(buddy.start_addr);

                    let new_combined_node_node = AllocatorNode {
                        free_list_index: node.free_list_index + 1,
                        start_addr,
                        size: node.size * 2,
                        next_in_free_list: None,
                    };

                    unsafe {
                        self.recursively_dealloc_and_connect_buddy_nodes(
                            new_combined_node_node,
                            dealloc_ptr,
                        )
                    };
                    return;
                }
                None => {}
            }
        }
        // end loop

        let free_list_index = node.free_list_index;
        assert_eq!(
            (dealloc_ptr as usize) % core::mem::align_of::<AllocatorNode>(),
            0,
            "dealloc_ptr is misaligned for AllocatorNode"
        );
        assert!(!dealloc_ptr.is_null(), "dealloc_ptr is null");

        let heap_alloc_node = unsafe {
            let node_pointer = node.start_addr as *mut AllocatorNode;

            node_pointer.write(node);

            &mut *node_pointer
        };

        check_pointer_safety(heap_alloc_node, "heap_alloc_node");

        self.add_to_free_list(free_list_index, heap_alloc_node);
    }
}
pub fn get_clamped_size_from_layout(layout: Layout) -> usize {
    layout.size().next_power_of_two().max(MIN_SIZE)
}

unsafe impl GlobalAlloc for Locked<BuddyAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let size = get_clamped_size_from_layout(layout);

        //
        let free_list_index = free_list_index(size);
        // debug!(
        //     "alloc: ========================================\n{:?}",
        //     layout, /* allocator.free_list */
        // );

        let node_to_assign = match allocator.free_list[free_list_index].take() {
            Some(free_node) => {
                allocator.free_list[free_list_index] = free_node.next_in_free_list.take();
                free_node
            }
            None => match unsafe { allocator.get_new_node_of_size(free_list_index) } {
                Some(new_node) => new_node,
                None => {
                    return ptr::null_mut(); // out of memory
                }
            },
        };

        node_to_assign.start_addr as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        let size = get_clamped_size_from_layout(layout);
        let free_list_index = free_list_index(size);
        let node = AllocatorNode {
            start_addr: ptr as usize,
            size,
            free_list_index,
            next_in_free_list: None,
        };
        // debug!(
        //     "dealloc: {:?}  =====================================\n ",
        //     node
        // );

        unsafe { allocator.recursively_dealloc_and_connect_buddy_nodes(node, ptr) };

        // debug!("\n {:#?}", allocator.free_list);
    }
}
