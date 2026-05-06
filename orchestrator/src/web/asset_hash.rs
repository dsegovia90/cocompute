/// Compile-time content hashes for cache-busting static assets.
/// When a file changes and the crate is rebuilt, the hash updates automatically.

const fn djb2(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 5381;
    let mut i = 0;
    while i < bytes.len() {
        hash = hash.wrapping_mul(33).wrapping_add(bytes[i] as u32);
        i += 1;
    }
    hash
}

macro_rules! asset_hash {
    ($path:literal) => {{
        const BYTES: &[u8] = include_bytes!(concat!("../../static/", $path));
        const HASH: u32 = djb2(BYTES);
        HASH
    }};
}

pub struct StaticAsset {
    pub path: &'static str,
    pub version: u32,
}

impl StaticAsset {
    pub fn url(&self) -> String {
        format!("/static/{}?v={:08x}", self.path, self.version)
    }
}

pub static CSS: StaticAsset = StaticAsset {
    path: "output.css",
    version: asset_hash!("output.css"),
};

pub static JS_NETWORK: StaticAsset = StaticAsset {
    path: "network-anim.js",
    version: asset_hash!("network-anim.js"),
};

pub static ICONS: StaticAsset = StaticAsset {
    path: "icons.svg",
    version: asset_hash!("icons.svg"),
};
