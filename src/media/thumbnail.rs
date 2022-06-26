// This example demonstrates how to get a raw video frame at a given position
// and then rescale and store it with the image crate:

// {uridecodebin} - {videoconvert} - {appsink}

// The appsink enforces RGBx so that the image crate can use it. The sample
// layout is passed with the correct stride from GStreamer to the image crate as
// GStreamer does not necessarily produce tightly packed pixels, and in case of
// RGBx never.
use std::sync::mpsc::{sync_channel, Receiver, Sender, TryRecvError, TrySendError};

use anyhow::{Error, Result};
use derive_more::{Display, Error};
use druid::{piet::ImageFormat, ImageBuf};
use gst::{element_error, prelude::*};
use gstreamer as gst;
use gstreamer::Pipeline;
use gstreamer_app as gst_app;

use crate::gui::data::video::VideoError;

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
	src: String,
	error: String,
	debug: Option<String>,
	source: glib::Error,
}
pub struct Thumbnail {
	pub receiver: Receiver<ImageBuf>,
	pipeline: Pipeline,
	pub duration: u64,
}

impl Thumbnail {
	pub fn new(uri: &str, position: u64) -> Result<Self> {
		let (sender, receiver) = sync_channel(1);
		gst::init()?;
		// Create our pipeline from a pipeline description string.
		let pipeline = gst::parse_launch(&format!(
                    "uridecodebin uri={} ! videoconvert ! appsink name=sink caps=\"video/x-raw, format=BGRA\"",
            uri
        ))?
            .downcast::<gst::Pipeline>()
            .expect("Expected a gst::Pipeline");

		// Get access to the appsink element.
		let appsink = pipeline
			.by_name("sink")
			.expect("Sink element not found")
			.downcast::<gst_app::AppSink>()
			.expect("Sink element is expected to be an appsink!");

		// Don't synchronize on the clock, we only want a snapshot asap.
		appsink.set_property("sync", false);

		// Tell the appsink what format we want.
		// This can be set after linking the two objects, because format negotiation
		// between both elements will happen during pre-rolling of the pipeline.
		/*        appsink.set_caps(Some(&gst::Caps::new_simple(
					"video/x-raw",
					&[("format", &"RGBA"), ("width",&"320"), ("height", &"240")],
				)));
		*/
		let mut got_snapshot = false;

		// Getting data out of the appsink is done by setting callbacks on it.
		// The appsink will then call those handlers, as soon as data is available.
		appsink.set_callbacks(
			gst_app::AppSinkCallbacks::builder()
				// Add a handler to the "new-sample" signal.
				.new_sample(move |appsink| {
					// Pull the sample in question out of the appsink's buffer.
					let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
					let buffer = sample.buffer().ok_or_else(|| {
						element_error!(
							appsink,
							gst::ResourceError::Failed,
							("Failed to get buffer from appsink")
						);

						gst::FlowError::Error
					})?;

					// Make sure that we only get a single buffer
					if got_snapshot {
						return Err(gst::FlowError::Eos);
					}

					let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

					let pad = appsink.static_pad("sink").ok_or(gst::FlowError::Error)?;

					let caps = pad.current_caps().ok_or(gst::FlowError::Error)?;
					let s = caps.structure(0).ok_or(gst::FlowError::Error)?;
					let width = s.get::<i32>("width").map_err(|_| gst::FlowError::Error)?;
					let height = s.get::<i32>("height").map_err(|_| gst::FlowError::Error)?;
					// println!("W: {:?}, H: {:?}", width, height);
					// Send original and processed image.
					let image = ImageBuf::from_raw(
						map.as_slice().to_owned(),
						ImageFormat::RgbaSeparate,
						width as _,
						height as _,
					);
					match sender.try_send(image) {
						Ok(_) => {
							// Ok(gst::FlowSuccess::Ok)
						}
						Err(TrySendError::Full(_)) => {
							log::trace!("Channel is full, discarded frame");
							// Ok(gst::FlowSuccess::Ok)
						}
						Err(TrySendError::Disconnected(_)) => {
							log::debug!("Returning EOS in pipeline callback fn");
							// Err(gst::FlowError::Eos)
						}
					}
					Err(gst::FlowError::Eos)
				})
				.build(),
		);

		pipeline.set_state(gst::State::Paused)?;

		pipeline.state(gst::ClockTime::from_seconds(1)).0?;
		let duration = std::time::Duration::from_nanos(
			pipeline.query_duration::<gst::ClockTime>().ok_or(VideoError::Duration)?.nseconds(),
		)
		.as_secs();
		println!("duration: {}", duration);

		let bus = pipeline.bus().expect("Pipeline without bus. Shouldn't happen!");

		let mut seeked = false;

		for msg in bus.iter_timed(gst::ClockTime::NONE) {
			use gst::MessageView;

			match msg.view() {
				MessageView::AsyncDone(..) => {
					if !seeked {
						// AsyncDone means that the pipeline has started now and that we can seek
						println!("Got AsyncDone message, seeking to {}s", position);

						if pipeline
							.seek_simple(gst::SeekFlags::FLUSH, position * gst::ClockTime::SECOND)
							.is_err()
						{
							println!("Failed to seek, taking first frame");
						}

						pipeline.set_state(gst::State::Playing)?;
						seeked = true;
					} else {
						println!("Got second AsyncDone message, seek finished");
					}
				}
				MessageView::Eos(..) => {
					// The End-of-stream message is posted when the stream is done, which in our
					// case happens immediately after creating the thumbnail because we return
					// gst::FlowError::Eos then.
					println!("Got Eos message, done");
					break;
				}
				MessageView::Error(err) => {
					pipeline.set_state(gst::State::Null)?;
					return Err(ErrorMessage {
						src: msg
							.src()
							.map(|s| String::from(s.path_string()))
							.unwrap_or_else(|| String::from("None")),
						error: err.error().to_string(),
						debug: err.debug(),
						source: err.error(),
					}
					.into());
				}
				_ => (),
			}
		}
		Ok(Thumbnail { receiver, pipeline, duration })
	}
}

impl Drop for Thumbnail {
	fn drop(&mut self) {
		if self.pipeline.set_state(gst::State::Null).is_err() {
			log::error!("Could not stop pipeline");
		}
		log::debug!("Pipeline stopped!");
	}
}
