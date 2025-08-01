use std::borrow::Cow;

use nyquest_interface::{Part, PartBody};
use objc2::rc::Retained;
use objc2_foundation::NSData;

unsafe extern "C" {
    fn arc4random() -> u32;
}

pub fn generate_multipart_boundary() -> String {
    let [rnd1, rnd2] = unsafe { [arc4random(), arc4random()] };
    format!("----nyquest.boundary.{rnd1:08x}{rnd2:08x}")
}

fn quick_escape_header(key: &mut Cow<'static, str>, value: &mut Cow<'static, str>) {
    if key.contains(':') {
        *key = key.replace(':', "%3A").into();
    }
    static NEW_LINE: &[char] = &['\r', '\n'];
    for s in [key, value] {
        if s.contains(NEW_LINE) {
            *s = s.replace(NEW_LINE, "\\n").into();
        }
    }
}

fn estimate_multipart_body_size<S>(boundary: &str, parts: &[Part<S>]) -> usize {
    let size: usize = parts
        .iter()
        .map(|part| {
            let partbody = match &part.body {
                PartBody::Bytes { content } => content,
                _ => unimplemented!("nsurlsession multipart body type"),
            };
            80 + boundary.len()
                + part.name.len()
                + part.filename.as_ref().map(|s| s.len()).unwrap_or_default()
                + part.content_type.len()
                + part
                    .headers
                    .iter()
                    .map(|(k, v)| k.len() + v.len() + 4)
                    .sum::<usize>()
                + partbody.len()
        })
        .sum();
    size + boundary.len() + 6
}

pub fn generate_multipart_body<S>(boundary: &str, parts: Vec<Part<S>>) -> Retained<NSData> {
    let mut body = Vec::with_capacity(estimate_multipart_body_size(boundary, &parts));

    for part in parts {
        let partbody = match part.body {
            PartBody::Bytes { content } => content,
            _ => unimplemented!("nsurlsession multipart body type"),
        };
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"");
        body.extend_from_slice(part.name.as_bytes());
        body.extend_from_slice(b"\"");
        if let Some(mut filename) = part.filename {
            body.extend_from_slice(b"; filename=\"");
            const STRIPPED_CHARS: &[char] = &['"', '\\', '/'];
            if filename.contains(STRIPPED_CHARS) {
                filename = filename.replace(STRIPPED_CHARS, "_").into();
            }
            body.extend_from_slice(filename.as_bytes());
            body.extend_from_slice(b"\"");
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(b"Content-Type: ");
        body.extend_from_slice(part.content_type.as_bytes());
        body.extend_from_slice(b"\r\n");
        for (mut k, mut v) in part.headers {
            quick_escape_header(&mut k, &mut v);
            body.extend_from_slice(k.as_bytes());
            body.extend_from_slice(b": ");
            body.extend_from_slice(v.as_bytes());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(&partbody);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");

    unsafe { NSData::dataWithBytes_length(body.as_ptr() as *const _, body.len() as _) }
}
