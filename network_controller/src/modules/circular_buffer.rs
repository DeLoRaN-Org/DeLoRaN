#[derive(Debug)]
pub struct CircularBuffer<T, const BUFFER_SIZE: usize> {
    buffer: [Option<T>; BUFFER_SIZE],
    head: usize,
    tail: usize,
    size: usize,
}

impl<T, const BUFFER_SIZE: usize> Default for CircularBuffer<T, BUFFER_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const BUFFER_SIZE: usize> CircularBuffer<T, BUFFER_SIZE> {
    const ARRAY_DEFAULT_VALUE: Option<T> = None;
    pub fn new() -> Self {
        Self {
            buffer: [Self::ARRAY_DEFAULT_VALUE; BUFFER_SIZE],
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    pub fn enqueue(&mut self, item: T) {
        if self.size == BUFFER_SIZE {
            // Buffer is full, overwrite the oldest element
            self.tail = (self.tail + 1) % BUFFER_SIZE;
        } else {
            self.size += 1;
        }
        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % BUFFER_SIZE;
    }

    pub fn dequeue(&mut self) -> Option<T> {
        if self.size == 0 {
            return None; // Buffer is empty
        }
        let item = std::mem::take(&mut self.buffer[self.tail]);
        self.tail = (self.tail + 1) % BUFFER_SIZE;
        self.size -= 1;
        item
    }

    pub fn is_full(&self) -> bool {
        self.size == BUFFER_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn capacity(&self) -> usize {
        BUFFER_SIZE
    }

    pub fn clear(&mut self) {
        self.buffer = [Self::ARRAY_DEFAULT_VALUE; BUFFER_SIZE];
        self.head = 0;
        self.tail = 0;
        self.size = 0;
    }

    pub fn iter(&self) -> CircularBufferIterator<T,BUFFER_SIZE> {
        CircularBufferIterator {
            buffer: self,
            index: 0,
        }
    }
    
    pub fn iter_mut(&mut self) -> CircularBufferIterator<T,BUFFER_SIZE> {
        CircularBufferIterator {
            buffer: self,
            index: 0,
        }
    }
}


pub struct CircularBufferIterator<'a, T, const BUFFER_SIZE: usize> {
    buffer: &'a CircularBuffer<T, BUFFER_SIZE>,
    index: usize,
}

pub struct CircularBufferMutIterator<'a, T, const BUFFER_SIZE: usize> {
    buffer: &'a mut CircularBuffer<T, BUFFER_SIZE>,
    index: usize,
}

pub struct CircularBufferIntoIterator<T, const BUFFER_SIZE: usize> {
    buffer: CircularBuffer<T, BUFFER_SIZE>,
    index: usize,
}




impl <'a, T, const BUFFER_SIZE: usize> IntoIterator for &'a CircularBuffer<T, BUFFER_SIZE> {
    type Item = &'a T;
    type IntoIter = CircularBufferIterator<'a, T, BUFFER_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        CircularBufferIterator {
            buffer: self,
            index: 0,
        }
    }
}

impl <'a, T, const BUFFER_SIZE: usize> IntoIterator for &'a mut CircularBuffer<T, BUFFER_SIZE> {
    type Item = &'a mut T;
    type IntoIter = CircularBufferMutIterator<'a, T, BUFFER_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        CircularBufferMutIterator {
            buffer: self,
            index: 0,
        }
    }
}


impl<'a, T, const BUFFER_SIZE: usize > Iterator for CircularBufferMutIterator<'a, T, BUFFER_SIZE> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.buffer.size {
            let item = unsafe { &mut *self.buffer.buffer.as_mut_ptr().add((self.buffer.tail + self.index) % BUFFER_SIZE) };
            self.index += 1;
            item.as_mut()
        } else {
            None
        }
    }
}

impl<'a, T, const BUFFER_SIZE: usize > Iterator for CircularBufferIterator<'a, T, BUFFER_SIZE> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.buffer.size {
            let item = &self.buffer.buffer[(self.buffer.tail + self.index) % BUFFER_SIZE];
            self.index += 1;
            item.as_ref()
        } else {
            None
        }
    }
}

impl <T, const BUFFER_SIZE: usize> IntoIterator for CircularBuffer<T, BUFFER_SIZE> {
    type Item = T;
    type IntoIter = CircularBufferIntoIterator<T, BUFFER_SIZE>;

    fn into_iter(self) -> Self::IntoIter {
        CircularBufferIntoIterator {
            buffer: self,
            index: 0,
        }
    }
}

impl<T, const BUFFER_SIZE: usize> Iterator for CircularBufferIntoIterator<T, BUFFER_SIZE> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.buffer.size {
            let item = std::mem::take(&mut self.buffer.buffer[(self.buffer.tail + self.index) % BUFFER_SIZE]);
            self.index += 1;
            item
        } else {
            None
        }
    }
}


#[test]

fn iterator_test() {
    let mut buffer = CircularBuffer::<u32, 4>::new();
    buffer.enqueue(10);
    buffer.enqueue(20);
    buffer.enqueue(30);
    buffer.enqueue(40);

    let mut v = buffer.dequeue();
    assert_eq!(v, Some(10));

    v = buffer.dequeue();
    assert_eq!(v, Some(20));
    v = buffer.dequeue();
    assert_eq!(v, Some(30));
    v = buffer.dequeue();
    assert_eq!(v, Some(40));
    v = buffer.dequeue();
    assert_eq!(v, None);
    v = buffer.dequeue();
    assert_eq!(v, None);
    v = buffer.dequeue();
    assert_eq!(v, None);

    buffer.enqueue(10);
    buffer.enqueue(20);
    buffer.enqueue(30);
    buffer.enqueue(40);
    buffer.enqueue(50);


    for v in &mut buffer {
        println!("v: {:?}", v);
        *v = 100;
    }   

    for v in &buffer {
        println!("v: {:?}", v);
        assert_eq!(*v, 100);
    }
}