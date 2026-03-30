use super::store::CounterStore;
use crate::Provider;
use conject::injectable;

#[injectable]
pub struct CounterService {
    store: &'static CounterStore,
}

impl CounterService {
    pub fn clear(&self) {
        self.store.update(|x| *x = 0);
    }
    pub fn decrement(&self) {
        self.store.update(|x| *x -= 1);
    }
    pub fn increment(&self) {
        self.store.update(|x| *x += 1);
    }
}

pub fn counter_service() -> CounterService {
    Provider::inject::<CounterService>()
}
