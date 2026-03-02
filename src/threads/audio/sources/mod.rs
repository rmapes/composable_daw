/*
 * Sources for the audio thread.
 *
 * Sources are responsible for generating audio data for output.
 * They will generate bytes on demand, using a pull model from the end of the pipeline.
 */

pub mod synth;
