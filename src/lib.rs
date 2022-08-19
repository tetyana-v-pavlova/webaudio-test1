mod utils;

use std::borrow::Borrow;
use std::cell::{Ref, RefCell};

use std::rc::Rc;
use cfg_if::cfg_if;
use js_sys::ArrayBuffer;
use wasm_bindgen::prelude::*;
use log::{info, trace, warn, error};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, OscillatorType, Request, RequestInit, RequestMode, Response, window};

cfg_if! {
    if #[cfg(feature = "console_log")] {
        fn init_log() {
            use log::Level;
            console_log::init_with_level(Level::Trace).expect("error initializing log");
        }
    } else {
        fn init_log() {}
    }
}
// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;



thread_local! {
    static ON_CLICK:RefCell<Option<Closure<dyn FnMut(web_sys::MouseEvent)>>> = RefCell::new(None);

}

pub fn midi_to_freq(note: u8) -> f32 {
    27.5 * 2f32.powf((note as f32 - 21.0) / 12.0)
}

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}


#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    init_log();


    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let ctx = Rc::new(web_sys::AudioContext::new()?);

    let repinique = Repinique::new(Rc::clone(&ctx)).await?;

    let mut count = Rc::new(RefCell::new(0));


    ON_CLICK.with(|ref_cell| {
        let closure_on_click = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            trace!(">>>>>>>>>>>>>>>>>>click>>>>>>>>>>>>>>>>>>>");
            repinique.play();

            match document.get_element_by_id("count_div") {
                Some(element) => {
                    let mut ref_mut = count.borrow_mut();
                    *ref_mut +=1;

                    element.set_inner_html(format!("{}", ref_mut).as_str());
                }
                _ => {
                    trace!("no msg");
                }
            }

        }) as Box<dyn FnMut(web_sys::MouseEvent)>);


        if web_sys::window().unwrap().add_event_listener_with_callback("touchstart", &closure_on_click.as_ref().unchecked_ref()).is_err(){
            panic!("window.add_event_listener_with_callback click")
        }

        ref_cell.borrow_mut().replace(closure_on_click);
    });

    Ok(())
}



pub struct Repinique {
    ctx:Rc<AudioContext>,
    audio_buffer: AudioBuffer,
}



impl Repinique{


    pub async fn new(ctx:Rc<AudioContext>) -> Result<Repinique, JsValue> {
        // let source = ctx.create_buffer_source()?;
        let array_buffer = download_content_1(String::from("http://192.168.0.92:7777/sounds/repinique_0.mp3")).await?;

        let decoded_buffer = JsFuture::from(ctx.decode_audio_data(&array_buffer)?).await?;




        Ok(Self {
            ctx,
            audio_buffer: decoded_buffer.dyn_into()?
        })
    }

    pub fn play(&self){
        let  source =  AudioBufferSourceNode::new(&self.ctx).expect("");
        source.set_buffer(Some(&self.audio_buffer));
        source.connect_with_audio_node(&self.ctx.destination());
        source.start();

        source.start();
        trace!("playing repinique");
    }
}

#[wasm_bindgen]
pub async fn play_s1() -> Result<(), JsValue>  {
    // alert("AAAAAAAAAAAA");
    let ctx = web_sys::AudioContext::new()?;
    let source = ctx.create_buffer_source()?;

    let array_buffer = download_content_1(String::from("http://http://192.168.0.92:7777/sounds/repinique_0.mp3")).await?;
    let decoded_buffer = JsFuture::from(ctx.decode_audio_data(&array_buffer)?).await?;
    trace!("playing s2 decoded_buffer={:?}", decoded_buffer);
    let audio_buffer:AudioBuffer = decoded_buffer.dyn_into()?;
    source.set_buffer(Some(&audio_buffer));
    // source.connect(ctx.destination);
    // source.connect_with_audio_node_and_output()
    source.connect_with_audio_node(&ctx.destination());
    source.start();

    Ok(())
}

async fn download_content_1(url: String) -> Result<ArrayBuffer, JsValue> {
    //log(format!("download_content_1: url={:?}", url).as_str());
    trace!("download_content_1. 1");
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let window = web_sys::window().unwrap();
    trace!("download_content_1. 2");
    let resp_value = JsFuture::from(window.fetch_with_request(&Request::new_with_str_and_init(url.as_ref(), &opts)?)).await?;
    trace!("download_content_1. 3 resp_value={:?}", resp_value);

    let resp: Response = resp_value.dyn_into()?;
    //log(format!("download_content_1: resp.status()={:?}", resp.status()).as_str());

    if resp.status() != 200 {
        return Err(JsValue::from(0u32));
    }


    let buf_val = JsFuture::from(resp.array_buffer()?).await?;
    assert!(buf_val.is_instance_of::<ArrayBuffer>());

    // debug(&format!("download_content_1: got array_buf: {}", buf_val.is_instance_of::<ArrayBuffer>()), false);

    // let js_u8_array: js_sys::Uint8Array = js_sys::Uint8Array::new(&buf_val);
    //
    //
    // // log(&format!("download_content_1: js_u8_array ok {:?} ", js_u8_array));
    //
    // let content = js_u8_array.to_vec();
    // // log(format!("download_content_1: content size: {}", content.len()).as_str());


    // return Ok(buf_val.dyn_into());
    Ok(buf_val.dyn_into()?)
}


#[wasm_bindgen]
pub struct FmOsc {
    ctx: AudioContext,
    /// The primary oscillator.  This will be the fundamental frequency
    primary: web_sys::OscillatorNode,

    /// Overall gain (volume) control
    gain: web_sys::GainNode,

    /// Amount of frequency modulation
    fm_gain: web_sys::GainNode,

    /// The oscillator that will modulate the primary oscillator's frequency
    fm_osc: web_sys::OscillatorNode,

    /// The ratio between the primary frequency and the fm_osc frequency.
    ///
    /// Generally fractional values like 1/2 or 1/4 sound best
    fm_freq_ratio: f32,

    fm_gain_ratio: f32,
}

impl Drop for FmOsc {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

#[wasm_bindgen]
impl FmOsc {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<FmOsc, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        // Create our web audio objects.
        let primary = ctx.create_oscillator()?;
        let fm_osc = ctx.create_oscillator()?;
        let gain = ctx.create_gain()?;
        let fm_gain = ctx.create_gain()?;

        // Some initial settings:
        primary.set_type(OscillatorType::Sine);
        primary.frequency().set_value(440.0); // A4 note
        gain.gain().set_value(0.0); // starts muted
        fm_gain.gain().set_value(0.0); // no initial frequency modulation
        fm_osc.set_type(OscillatorType::Sine);
        fm_osc.frequency().set_value(0.0);

        // Connect the nodes up!

        // The primary oscillator is routed through the gain node, so that
        // it can control the overall output volume.
        primary.connect_with_audio_node(&gain)?;

        // Then connect the gain node to the AudioContext destination (aka
        // your speakers).
        gain.connect_with_audio_node(&ctx.destination())?;

        // The FM oscillator is connected to its own gain node, so it can
        // control the amount of modulation.
        fm_osc.connect_with_audio_node(&fm_gain)?;

        // Connect the FM oscillator to the frequency parameter of the main
        // oscillator, so that the FM node can modulate its frequency.
        fm_gain.connect_with_audio_param(&primary.frequency())?;

        // Start the oscillators!
        primary.start()?;
        fm_osc.start()?;

        Ok(FmOsc {
            ctx,
            primary,
            gain,
            fm_gain,
            fm_osc,
            fm_freq_ratio: 0.0,
            fm_gain_ratio: 0.0,
        })
    }

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    #[wasm_bindgen]
    pub fn set_gain(&self, mut gain: f32) {
        if gain > 1.0 {
            gain = 1.0;
        }
        if gain < 0.0 {
            gain = 0.0;
        }
        self.gain.gain().set_value(gain);
    }

    #[wasm_bindgen]
    pub fn set_primary_frequency(&self, freq: f32) {
        self.primary.frequency().set_value(freq);

        // The frequency of the FM oscillator depends on the frequency of the
        // primary oscillator, so we update the frequency of both in this method.
        self.fm_osc.frequency().set_value(self.fm_freq_ratio * freq);
        self.fm_gain.gain().set_value(self.fm_gain_ratio * freq);
    }

    #[wasm_bindgen]
    pub fn set_note(&self, note: u8) {
        let freq = midi_to_freq(note);
        self.set_primary_frequency(freq);
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_amount(&mut self, amt: f32) {
        self.fm_gain_ratio = amt;

        self.fm_gain
            .gain()
            .set_value(self.fm_gain_ratio * self.primary.frequency().value());
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_frequency(&mut self, amt: f32) {
        self.fm_freq_ratio = amt;
        self.fm_osc
            .frequency()
            .set_value(self.fm_freq_ratio * self.primary.frequency().value());
    }
}
