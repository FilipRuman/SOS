use super::{StaticTask, TaskId};
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::Waker;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref TASK_SPAWNER: TaskSpawner = TaskSpawner::new();
}

pub struct TaskSpawner {
    pub tasks: Mutex<BTreeMap<TaskId, StaticTask>>,
    pub task_queue: Arc<ArrayQueue<TaskId>>,
}
impl TaskSpawner {
    pub fn new() -> TaskSpawner {
        TaskSpawner {
            tasks: Mutex::new(BTreeMap::new()),
            task_queue: Arc::new(ArrayQueue::new(20)),
        }
    }
    pub fn spawn(&self, task: StaticTask) {
        // to prevent any dumb shit from happening eg.(deadlocks)
        x86_64::instructions::interrupts::disable();
        let task_id = task.id;
        if self.tasks.lock().insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
        x86_64::instructions::interrupts::enable();
    }
}

pub struct Executor {
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            waker_cache: BTreeMap::new(),
        }
    }
}

use core::task::{Context, Poll};

impl Executor {
    pub fn run(&mut self) -> ! {
        loop {
            // to prevent any dumb shit from happening eg.(deadlocks)
            x86_64::instructions::interrupts::disable();
            self.run_ready_tasks();
            x86_64::instructions::interrupts::enable_and_hlt();
        }
    }

    fn run_ready_tasks(&mut self) {
        // destructure `self` to avoid borrow checker errors

        let Self { waker_cache } = self;

        let mut tasks = TASK_SPAWNER.tasks.lock();

        while let Some(task_id) = TASK_SPAWNER.task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => {
                    continue;
                }
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, TASK_SPAWNER.task_queue.clone()));
            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }
}
struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}
impl TaskWaker {
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}
impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
}
use alloc::task::Wake;

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
