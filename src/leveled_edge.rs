use crate::debounce::*;
use embedded_hal::digital::v2::InputPin;
use esp_idf_hal::gpio::{GpioPin, Input, InterruptType, Pin};
use esp_idf_sys::{esp, gpio_set_intr_type, EspError, ESP_ERR_INVALID_STATE, ESP_OK};

/// A leveled edge interrupt handle
pub struct LeveledEdge<Debouncer, Func>
where
    Debouncer: Debounce,
    Func: FnMut(bool) -> (),
{
    gpio: GpioPin<Input>,
    pin_state: bool,
    debouncer: Debouncer,
    callback: *mut Func,
}

impl<Debouncer, Func> LeveledEdge<Debouncer, Func>
where
    Debouncer: Debounce,
    Func: FnMut(bool),
{
    /// Create a new instance of `LeveledEdge`
    /// debouncer: The debouncer to use, if no debouncer is needed, use `NoDebounce`
    /// callback: The callback to call when the pin changes
    /// arg: The argument to pass to the callback
    ///
    /// The callback will be called with the following arguments:
    /// - `true` if the pin is high
    /// - `false` if the pin is low
    ///
    /// # Example - creating an interrupt on pin 4 button to light led on pin 18
    /// ```
    /// use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
    /// use esp_idf_hal::{delay::TickType, gpio::*, prelude::Peripherals};
    ///
    /// let pins = Peripherals::take().unwrap().pins;
    ///
    /// let mut led = pins.gpio18.into_output().unwrap().degrade();
    /// led.set_low().unwrap();
    ///
    /// // create the butoon interrupt handler
    /// let _button = LeveledEdge::new(
    ///     //button pin
    ///     pins.gpio4.into_input().unwrap().degrade(),
    ///     //debounce filter with 20ms
    ///     FilterDebounce::new(Duration::from_millis(20)),
    ///     //callback, runs from interrupt handler.. beware of blocking
    ///     |state| {
    ///         led.set_state(PinState::from(!state)).unwrap();
    ///       },
    ///     )
    /// .unwrap();
    ///
    /// loop {
    ///      // don't let the watchdog get you
    ///      vTaskDelay(1000);
    /// }
    ///
    /// ```
    ///     
    ///
    pub fn new(
        gpio: GpioPin<Input>,
        debouncer: Debouncer,
        callback: Func,
    ) -> Result<Box<Self>, EspError> {
        let pin_state = gpio.is_high()?;

        let mut this = Box::new(LeveledEdge {
            gpio,
            pin_state,
            debouncer,
            callback: Box::into_raw(Box::new(callback)),
        });

        this.install_isr()?;

        Ok(this)
    }

    ///Install the interrupt handler on the pin supplied
    fn install_isr(&mut self) -> Result<(), EspError> {
        let next_intr = match self.pin_state {
            true => InterruptType::LowLevel,
            false => InterruptType::HighLevel,
        };

        esp!(unsafe { gpio_set_intr_type(self.gpio.pin(), next_intr.into()) })?;

        unsafe {
            match esp_idf_sys::gpio_install_isr_service(0) {
                ESP_OK | ESP_ERR_INVALID_STATE => {}
                err => return Err(EspError::from(err).unwrap()),
            }
        }

        esp!(unsafe {
            esp_idf_sys::gpio_isr_handler_add(
                self.gpio.pin(),
                Some(Self::irq_handler),
                self as *mut Self as *mut _,
            )
        })?;

        Ok(())
    }

    /// This is the real interrupt handler being run on interrupt,
    /// It will debounce the pin, and if valid will call the callback with the current pins state and arg
    /// After that he will toggle the interrupt type to the next one, and wait for the next interrupt
    #[inline(always)]
    #[link_section = ".iram1.leveled_edge"]
    unsafe extern "C" fn irq_handler(this: *mut esp_idf_sys::c_types::c_void) {
        let this: &mut LeveledEdge<Debouncer, Func> = &mut *(this as *mut _);

        // disable the interrupt, maybe not really needed..
        esp_idf_sys::gpio_intr_disable(this.gpio.pin());

        //toggle the pin state
        this.pin_state = !this.pin_state;

        //debounce the pin, call callback if debounced
        if this.debouncer.is_isr_valid() {
            (*this.callback)(this.pin_state);
        }

        //toggle the interrupt type
        this.toggle_interrupt_trigger();
        esp_idf_sys::gpio_intr_enable(this.gpio.pin());
    }

    ///toggle the interrupt trigger between high and low level
    fn toggle_interrupt_trigger(&mut self) {
        let next_intr = match self.pin_state {
            true => InterruptType::LowLevel,
            false => InterruptType::HighLevel,
        };

        unsafe {
            match gpio_set_intr_type(self.gpio.pin(), next_intr.into()) {
                ESP_OK => {}
                err => panic!("gpio_set_intr_type failed: {:?}", err),
            }
        }
    }
}
