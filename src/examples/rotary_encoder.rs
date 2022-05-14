use std::mem::size_of;
use std::time::Duration;

use embedded_hal::digital::v2::InputPin;
use esp_idf_hal::{delay::TickType, gpio::*, prelude::Peripherals};
use esp_idf_sys::BaseType_t; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::{self as _, c_types::c_void, EspError};
use std::sync::Arc;

use esp_leveled_edge::{FilterDebounce, LeveledEdge};

struct DirectionQueue {
    pub handle: esp_idf_sys::QueueHandle_t,
}

impl DirectionQueue {
    const LENGTH: u32 = 100;
    const ITEM_SIZE: u32 = size_of::<RotaryEncoder>() as u32;

    fn new() -> Self {
        unsafe {
            Self {
                handle: esp_idf_sys::xQueueGenericCreate(Self::LENGTH, Self::ITEM_SIZE, 0),
            }
        }
    }
}

unsafe impl Send for DirectionQueue {}

type DirectionPinAndQueue = (GpioPin<Input>, DirectionQueue);

struct RotaryEncoder {
    dt_and_queue: Arc<DirectionPinAndQueue>,
    _interrupt: Box<LeveledEdge<FilterDebounce, Box<dyn FnMut(bool) -> ()>>>,
}

pub enum RotaryDirection {
    Clockwise,
    CounterClockwise,
}

impl RotaryEncoder {
    pub fn new(clk: GpioPin<Input>, dt: GpioPin<Input>) -> Result<Self, EspError> {
        let stream = DirectionQueue::new();

        let dt_and_queue = Arc::new((dt, stream));
        let dt_and_queue_clone = dt_and_queue.clone();

        let closre: Box<dyn FnMut(bool) -> ()> = Box::new(move |state| {
            let mut direction = if state != dt_and_queue_clone.0.is_high().unwrap() {
                RotaryDirection::Clockwise
            } else {
                RotaryDirection::CounterClockwise
            };

            let mut higher_prio_task_woken: BaseType_t = Default::default();
            //write direction to stream
            unsafe {
                esp_idf_sys::xQueueGenericSendFromISR(
                    dt_and_queue_clone.1.handle,
                    &mut direction as *mut _ as *mut c_void,
                    &mut higher_prio_task_woken as *mut _,
                    0,
                );
            }

            if higher_prio_task_woken != 0 {
                unsafe { esp_idf_sys::vPortEvaluateYieldFromISR(0) }
            }
        });

        let _interrupt =
            LeveledEdge::new(clk, FilterDebounce::new(Duration::from_millis(20)), closre)?;

        let rotary = RotaryEncoder {
            dt_and_queue,
            _interrupt,
        };

        Ok(rotary)
    }

    pub fn wait_on_direction(&mut self, timeout: TickType) -> Option<RotaryDirection> {
        let mut direction = RotaryDirection::Clockwise;
        let dir = &mut direction as *mut _ as *mut c_void;
        let err: bool =
            unsafe { esp_idf_sys::xQueueReceive(self.dt_and_queue.1.handle, dir, timeout.0) > 0 };

        match err {
            true => Some(direction),
            false => None,
        }
    }
}

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    println!("Encoder example!");

    let pins = Peripherals::take().unwrap().pins;
    let mut rotary = RotaryEncoder::new(
        pins.gpio25.into_input().unwrap().degrade(),
        pins.gpio27.into_input().unwrap().degrade(),
    )
    .unwrap();

    loop {
        match rotary.wait_on_direction(Duration::from_millis(10000).into()) {
            Some(RotaryDirection::Clockwise) => println!("Clockwise"),
            Some(RotaryDirection::CounterClockwise) => println!("CounterClockwise"),
            None => {}
        }
    }
}
