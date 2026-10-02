//! Removes identifying metadata from an image before it's written to a
//! Table — THREAT_MODEL.md's "Photos keep their metadata" finding, fixed
//! here in the core crate so every client (Tauri, CLI, anything else)
//! goes through it rather than each UI remembering to.
//!
//! **Allowlist, not denylist.** Each format keeps only the parts needed
//! to draw the picture correctly — pixel data, how to decode it, and
//! color information — and drops everything else, including parts this
//! code has never heard of. Metadata hides in more places than anyone
//! lists (EXIF, XMP, IPTC, comments, text chunks, timestamps, a second
//! image appended after the first one ends), so the safe default for an
//! unknown part is to drop it, not keep it.
//!
//! **Lossless.** Pixels are never decoded or re-encoded; whole segments
//! and chunks are copied or dropped as they are. A stripped image is
//! byte-for-byte the original minus its metadata.
//!
//! **Orientation survives.** Phones store "rotate this 90°" as an EXIF
//! tag, not by rotating pixels — dropping it outright makes photos show
//! sideways. For JPEG, the one tag kept is the orientation, written into
//! a fresh, minimal EXIF block that carries nothing else.
//!
//! **Fails closed.** Only JPEG, PNG and WebP are understood. Anything
//! else (HEIC, GIF, AVIF, TIFF, or bytes that aren't an image) is
//! rejected rather than stored with whatever it carries. Format is
//! detected from the bytes themselves, never from a caller-supplied
//! content type.

use anyhow::{bail, ensure, Context, Result};

/// Strips identifying metadata from `bytes`, returning the cleaned image.
/// Errors on any format this module can't clean (see the module doc).
pub fn strip_metadata(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        strip_jpeg(bytes).context("couldn't read this JPEG")
    } else if bytes.starts_with(PNG_SIGNATURE) {
        strip_png(bytes).context("couldn't read this PNG")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        strip_webp(bytes).context("couldn't read this WebP")
    } else {
        bail!(
            "this image format can't be checked for location and other hidden \
             metadata, so it wasn't uploaded — use a JPEG, PNG or WebP"
        )
    }
}

// ── JPEG ────────────────────────────────────────────────────────────────

fn strip_jpeg(bytes: &[u8]) -> Result<Vec<u8>> {
    // Pieces kept, in order: whole segments (marker included) and the
    // compressed scan data between them.
    let mut kept: Vec<&[u8]> = Vec::new();
    let mut orientation: Option<u16> = None;
    let mut i = 2; // past SOI

    loop {
        // Every marker is 0xFF followed by a code; any number of 0xFF
        // fill bytes may precede it.
        ensure!(i < bytes.len() && bytes[i] == 0xFF, "expected a marker at byte {i}");
        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        ensure!(i < bytes.len(), "file ends inside a marker");
        let code = bytes[i];
        let start = i - 1;
        i += 1;

        match code {
            // EOI: the image ends here. Anything after it — phones append
            // whole second images (depth maps, previews) with their own
            // EXIF — is dropped by not reading further.
            0xD9 => {
                kept.push(&bytes[start..i]);
                break;
            }
            // Standalone markers with no length: TEM, RST0–7.
            0x01 | 0xD0..=0xD7 => kept.push(&bytes[start..i]),
            _ => {
                ensure!(i + 2 <= bytes.len(), "segment length missing");
                let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
                ensure!(len >= 2 && i + len <= bytes.len(), "segment runs past end of file");
                let segment = &bytes[start..i + len];
                let payload = &bytes[i + 2..i + len];
                i += len;

                let keep = match code {
                    0xE0 => payload.starts_with(b"JFIF\0") || payload.starts_with(b"JFXX\0"),
                    0xE1 => {
                        if let Some(rest) = payload.strip_prefix(b"Exif\0\0") {
                            orientation = orientation.or_else(|| exif_orientation(rest));
                        }
                        false // EXIF and XMP both live in APP1; neither is kept
                    }
                    0xE2 => payload.starts_with(b"ICC_PROFILE\0"), // color, not identity
                    0xEE => payload.starts_with(b"Adobe"),         // color transform flag
                    0xE3..=0xEF | 0xFE => false,                   // other APPn, comments
                    // Frame, table and scan headers: what decoding needs.
                    0xC0..=0xCF | 0xDA..=0xDF => true,
                    _ => bail!("unsupported JPEG segment 0x{code:02X}"),
                };
                if keep {
                    kept.push(segment);
                }

                if code == 0xDA {
                    // Start of scan: compressed data follows until the
                    // next real marker. Inside it, 0xFF is always followed
                    // by 0x00 (a stuffed byte) or an RST marker.
                    let scan_start = i;
                    while i + 1 < bytes.len() {
                        if bytes[i] == 0xFF && bytes[i + 1] != 0x00 && !(0xD0..=0xD7).contains(&bytes[i + 1]) {
                            break;
                        }
                        i += 1;
                    }
                    if i + 1 >= bytes.len() {
                        // Truncated file with no EOI: keep what decodes,
                        // add nothing.
                        kept.push(&bytes[scan_start..]);
                        break;
                    }
                    kept.push(&bytes[scan_start..i]);
                }
            }
        }
    }

    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&[0xFF, 0xD8]);
    let minimal_exif = orientation.filter(|o| (2..=8).contains(o)).map(orientation_app1);
    // The minimal orientation EXIF goes right after a leading JFIF APP0
    // (JFIF requires APP0 first), otherwise right after SOI.
    let insert_after = usize::from(kept.first().is_some_and(|s| s.get(1) == Some(&0xE0)));
    for (n, piece) in kept.iter().enumerate() {
        if n == insert_after {
            if let Some(exif) = &minimal_exif {
                out.extend_from_slice(exif);
            }
        }
        out.extend_from_slice(piece);
    }
    Ok(out)
}

/// Reads the Orientation tag (0x0112) from a TIFF-structured EXIF block's
/// first IFD. Any malformation just means "no orientation" — this is
/// salvage, not validation.
fn exif_orientation(tiff: &[u8]) -> Option<u16> {
    let big = match tiff.get(0..2)? {
        b"MM" => true,
        b"II" => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let b: [u8; 2] = tiff.get(at..at + 2)?.try_into().ok()?;
        Some(if big { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let b: [u8; 4] = tiff.get(at..at + 4)?.try_into().ok()?;
        Some(if big { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    };
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    (0..count).find_map(|n| {
        let entry = ifd + 2 + n * 12;
        (u16_at(entry)? == 0x0112 && u16_at(entry + 2)? == 3).then(|| u16_at(entry + 8))?
    })
}

/// A complete APP1 segment holding an EXIF block with exactly one tag:
/// Orientation. Big-endian TIFF, one IFD, one entry, no next IFD.
fn orientation_app1(orientation: u16) -> Vec<u8> {
    let mut payload = b"Exif\0\0MM\0\x2A\0\0\0\x08".to_vec();
    payload.extend_from_slice(&1u16.to_be_bytes()); // one entry
    payload.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    payload.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    payload.extend_from_slice(&1u32.to_be_bytes()); // count
    payload.extend_from_slice(&orientation.to_be_bytes());
    payload.extend_from_slice(&[0, 0]); // value padding
    payload.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    let mut segment = vec![0xFF, 0xE1];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(&payload);
    segment
}

// ── PNG ─────────────────────────────────────────────────────────────────

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Chunks that describe pixels, color or animation — nothing about who
/// made the image, where, or when. Notably absent: `eXIf`, `tEXt`,
/// `zTXt`, `iTXt` (where XMP lives) and `tIME`.
const PNG_KEEP: &[&[u8; 4]] = &[
    b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"cHRM", b"gAMA", b"iCCP", b"sBIT",
    b"sRGB", b"cICP", b"mDCV", b"cLLI", b"bKGD", b"hIST", b"pHYs", b"sPLT", b"acTL",
    b"fcTL", b"fdAT",
];

fn strip_png(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = PNG_SIGNATURE.to_vec();
    let mut i = PNG_SIGNATURE.len();
    loop {
        ensure!(i + 8 <= bytes.len(), "file ends before IEND");
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into()?) as usize;
        let kind: &[u8; 4] = bytes[i + 4..i + 8].try_into()?;
        let end = i.checked_add(12).and_then(|n| n.checked_add(len)).context("chunk too large")?;
        ensure!(end <= bytes.len(), "chunk runs past end of file");
        if PNG_KEEP.contains(&kind) {
            out.extend_from_slice(&bytes[i..end]); // CRC travels with it, unchanged
        } else if kind[0].is_ascii_uppercase() {
            // An unknown *critical* chunk: dropping it would break the
            // image, keeping it might keep metadata. Refuse.
            bail!("unsupported PNG chunk {}", String::from_utf8_lossy(kind));
        }
        i = end;
        if kind == b"IEND" {
            return Ok(out); // anything after IEND is dropped
        }
    }
}

// ── WebP ────────────────────────────────────────────────────────────────

/// Image data, alpha, animation and color profile. Notably absent:
/// `EXIF` and `XMP `.
const WEBP_KEEP: &[&[u8; 4]] = &[b"VP8 ", b"VP8L", b"VP8X", b"ALPH", b"ANIM", b"ANMF", b"ICCP"];

fn strip_webp(bytes: &[u8]) -> Result<Vec<u8>> {
    let riff_len = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
    let end = riff_len.checked_add(8).context("RIFF size too large")?;
    ensure!(end <= bytes.len(), "RIFF runs past end of file");
    let mut body = b"WEBP".to_vec();
    let mut i = 12;
    while i < end {
        ensure!(i + 8 <= end, "chunk header runs past end of file");
        let kind: &[u8; 4] = bytes[i..i + 4].try_into()?;
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into()?) as usize;
        let padded = len + (len & 1);
        let next = i.checked_add(8 + padded).context("chunk too large")?;
        ensure!(next <= end, "chunk runs past end of file");
        if WEBP_KEEP.contains(&kind) {
            let at = body.len();
            body.extend_from_slice(&bytes[i..next]);
            if kind == b"VP8X" {
                ensure!(len >= 1, "empty VP8X chunk");
                body[at + 8] &= !(0x08 | 0x04); // clear the EXIF and XMP flags
            }
        }
        i = next;
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len())?.to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every fixture carries these strings inside its metadata (EXIF text,
    /// GPS area, XMP, comments, PNG text chunks, an EXIF block on an image
    /// appended after the JPEG's end). None may survive.
    const SECRETS: &[&[u8]] = &[
        b"SECRETDESC", b"SECRETPLACE", b"SECRETXMP", b"SECRETCOMMENT", b"SECRETTRAILER",
        b"SECRETTEXT", b"LEAKYCAM",
    ];

    fn assert_clean(original: &[u8]) -> Vec<u8> {
        assert!(SECRETS.iter().any(|s| contains(original, s)), "fixture lost its secrets");
        let stripped = strip_metadata(original).unwrap();
        for secret in SECRETS {
            assert!(!contains(&stripped, secret), "{} survived", String::from_utf8_lossy(secret));
        }
        // Stripping is a fixed point: a clean image passes through unchanged.
        assert_eq!(strip_metadata(&stripped).unwrap(), stripped);
        stripped
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn jpeg_loses_exif_xmp_comments_and_trailing_image_but_keeps_orientation() {
        let stripped = assert_clean(include_bytes!("../tests/fixtures/meta.jpg"));
        assert!(stripped.ends_with(&[0xFF, 0xD9]), "nothing after EOI");
        assert_eq!(stripped.windows(2).filter(|w| *w == [0xFF, 0xD9]).count(), 1);
        // The fixture's orientation (6, "rotate 90° clockwise") survives
        // as the only EXIF content, placed after the JFIF header.
        let exif = orientation_app1(6);
        assert!(contains(&stripped, &exif));
        assert_eq!(&stripped[2..4], &[0xFF, 0xE0], "JFIF APP0 still comes first");
    }

    #[test]
    fn png_loses_exif_text_xmp_and_time() {
        let stripped = assert_clean(include_bytes!("../tests/fixtures/meta.png"));
        for chunk in [b"eXIf", b"tEXt", b"iTXt", b"zTXt", b"tIME"] {
            assert!(!contains(&stripped, chunk));
        }
    }

    #[test]
    fn webp_loses_exif_and_xmp_and_clears_their_flags() {
        let stripped = assert_clean(include_bytes!("../tests/fixtures/meta.webp"));
        assert!(!contains(&stripped, b"EXIF") && !contains(&stripped, b"XMP "));
        let riff_len = u32::from_le_bytes(stripped[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_len + 8, stripped.len());
    }

    #[test]
    fn other_formats_are_refused_not_passed_through() {
        assert!(strip_metadata(b"GIF89a\x01\x00\x01\x00").is_err());
        assert!(strip_metadata(b"\0\0\0\x18ftypheic").is_err()); // iPhone HEIC
        assert!(strip_metadata(b"not an image at all").is_err());
        assert!(strip_metadata(&[]).is_err());
    }

    #[test]
    fn malformed_images_error_instead_of_panicking() {
        let jpeg = include_bytes!("../tests/fixtures/meta.jpg");
        let png = include_bytes!("../tests/fixtures/meta.png");
        let webp = include_bytes!("../tests/fixtures/meta.webp");
        for original in [&jpeg[..], &png[..], &webp[..]] {
            for cut in 0..original.len() {
                let _ = strip_metadata(&original[..cut]); // must not panic
            }
        }
    }
}
