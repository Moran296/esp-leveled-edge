# esp-leveled-edge
workaround for anyedge interrupts on esp32

### Oh hi there,

Did you ever try using AnyEdge interrupt on esp-idf to check if a button is pressed or not?
Did you notice the problem there?

Esp interrupts callback don't pass as paramter the state of the pin that caused the interrupts,
Also, reading the pin in the interrupt is not reliable..
Why?
becuase: https://www.espressif.com/sites/default/files/documentation/eco_and_workarounds_for_bugs_in_esp32_en.pdf section 3.14

From my experience, edge interrupts in esp32 are *problematic*.

There are all sorts of workarounds. Some people use timers to check the button after the interrupts.
some people just poll the pins (as dirty as it may sound).

### This crate
is a workaround too.

It provides an interrupt wrapper, using the level interrupts workaround to service callbacks with the pin state.

It means that you give me a Function and a debouncer (also provided by this crate) 
and I shoot the function with the state as a parameter after debouncing it and than toggle the interrupt trigger level.


### I tested it 
With buttons and rotary encoders. It is very reliable because you can't get stuck in the wrong state, even if the debouncing missed.
The level interrupt will make sure to fire until you toggle it to the other level...

### There is an example
With a cute rotary encoder driver in the examples folder, 

What? not enough for you?!
Here take this one,
It's a button example
```
    use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
    use esp_idf_hal::{delay::TickType, gpio::*, prelude::Peripherals};
    use esp_leveled_edge::{FilterDebounce, LeveledEdge};
    
fn main() {
    
    let pins = Peripherals::take().unwrap().pins;
    
    let mut led = pins.gpio18.into_output().unwrap().degrade();
    led.set_low().unwrap();
    
    // create the button interrupt handler  ---- THIS IS WHERE THE MAGIC HAPPENS
    let _button = LeveledEdge::new(
        pins.gpio4.into_input().unwrap().degrade(),      // the button
        FilterDebounce::new(Duration::from_millis(20)),  // the debouncer     
        |state| {                                        // the callback
            led.set_state(PinState::from(state)).unwrap();
          },
        )
    .unwrap();
    
    loop {
         // don't let the watchdog get you
         vTaskDelay(1000);
    }

}
```


#### Now go and think about it

