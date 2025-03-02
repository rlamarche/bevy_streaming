// SPDX-License-Identifier: MPL-2.0
#![allow(clippy::non_send_fields_in_send_ty, unused_doc_comments)]

use gst::glib;
use gstrswebrtc::signaller::Signallable;

mod imp;
mod protocol;

glib::wrapper! {
    pub struct UePsSignaller(ObjectSubclass<imp::Signaller>) @implements Signallable;
}

impl Default for UePsSignaller {
    fn default() -> Self {
        glib::Object::new()
    }
}
