use std::{cmp::Reverse, collections::BinaryHeap, time::Duration};

use lorawan_device::{communicator::Transmission, split_communicator::LoRaSender};
use tokio::{select, sync::mpsc::Receiver, time::Instant};

pub struct DownlinkSchedulerMessage<T> {
    pub transmission: Transmission,
    pub moment: Instant,
    pub additional_info: Option<T>,
}

impl <T> PartialEq for DownlinkSchedulerMessage<T> {
    fn eq(&self, other: &Self) -> bool {
        self.transmission == other.transmission && self.moment == other.moment
    }
}

impl <T> Eq for DownlinkSchedulerMessage<T> {}

impl <T> PartialOrd for DownlinkSchedulerMessage<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.moment.cmp(&other.moment))
    }
}

impl <T> Ord for DownlinkSchedulerMessage<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.moment.cmp(&other.moment)
    }
}

pub struct DownlinkScheduler<T: LoRaSender> {
    receiver: Receiver<DownlinkSchedulerMessage<T::OptionalInfo>>,
    downlink_communicator: T,
    message_storage: BinaryHeap<Reverse<DownlinkSchedulerMessage<T::OptionalInfo>>>
}



impl <T: LoRaSender> DownlinkScheduler<T> {
    pub fn new(downlink_communicator: T, receiver: Receiver<DownlinkSchedulerMessage<T::OptionalInfo>>) -> Self {
        Self {
            receiver,
            downlink_communicator,
            message_storage: BinaryHeap::new()
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                message = self.receiver.recv() => {
                    match message {
                        Some(t) => {
                            self.message_storage.push(Reverse(t,));
                        },
                        None => break,
                    }
                    
                },
                _ = tokio::time::sleep_until(self.message_storage.peek().map_or(Instant::now() + Duration::from_millis(100), |v| v.0.moment)), if self.message_storage.peek().is_some() => {
                    if let Some(head) = self.message_storage.pop() {
                        self.downlink_communicator.send(&head.0.transmission.payload, head.0.additional_info).await.unwrap();
                    }
                }
            }
        }
    }
}


