use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
    time,
};

use crate::config::Config;
use crate::store::merkle::MerkleTree;
use crate::store::KVEngineStoreTrait;

const FANOUT: &[u8] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ:_-./";

const DEFAULT_MAX_DEPTH: usize = 20;
const DEFAULT_LEAF_THRESHOLD: usize = 200;

pub struct SyncManager {
    store: Arc<Mutex<Box<dyn KVEngineStoreTrait + Send + Sync>>>,
    sync_interval: Duration,
    max_depth: usize,
    leaf_threshold: usize,
}

impl SyncManager {
    pub fn new_with_shared_store(
        cfg: &Config,
        store: Arc<Mutex<Box<dyn KVEngineStoreTrait + Send + Sync>>>,
    ) -> Self {
        Self {
            store,
            sync_interval: Duration::from_secs(cfg.sync_interval_seconds),
            max_depth: DEFAULT_MAX_DEPTH,
            leaf_threshold: DEFAULT_LEAF_THRESHOLD,
        }
    }

    /// One-shot: sync local with remote at host:port
    pub async fn sync_once(&self, host: &str, port: u16) -> Result<()> {
        let addr = format!("{host}:{port}");
        info!("SYNC (recursive Merkle) → {}", addr);
        self.sync_prefix_recursive(&addr, String::new(), 0).await
    }

    /// sync loop
    pub async fn start_sync_loop(&self, host: String, port: u16) {
        let mut interval = time::interval(self.sync_interval);
        let addr = format!("{host}:{port}");
        loop {
            interval.tick().await;
            if let Err(e) = self.sync_once(host.as_str(), port).await {
                log::warn!("background sync with {} failed: {}", addr, e);
            }
        }
    }

    // ─────────────────── Recursive by prefix ───────────────────

    /// Recursive by prefix: compare HASH(local,prefix) vs HASH(remote,prefix).
    /// If different: split by FANOUT; if at leaf: reconcile by SCAN prefix + GET.
    fn sync_prefix_recursive<'a>(
        &'a self,
        addr: &'a str,
        prefix: String,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // 1) Hash local/remote with prefix
            let local_hex = self.local_merkle_hex(&prefix).await?;
            let remote_hex = self.remote_hash_hex(addr, &prefix).await?;

            debug!("PREFIX {:?} depth={} local={} remote={}",
                   prefix, depth, &local_hex[..8], &remote_hex[..8]);

            if local_hex == remote_hex {
                // If equal → skip this branch.
                return Ok(());
            }

            // 2) If at leaf (max depth) → reconcile directly
            if depth >= self.max_depth {
                self.reconcile_leaf(addr, &prefix).await?;
                return Ok(());
            }

            // 3) If not at leaf → split recursively by next character in FANOUT
            for &ch in FANOUT {
                let mut sub = prefix.clone();
                sub.push(ch as char);
                self.sync_prefix_recursive(addr, sub, depth + 1).await?;
            }

            Ok(())
        })
    }

    // ─────────────────── Support function ───────────────────

    /// Compute Merkle root (hex) for a prefix on the LOCAL store (using existing MerkleTree).
    async fn local_merkle_hex(&self, prefix: &str) -> Result<String> {
        let (t, _map) = self.build_local_merkle_snapshot(prefix).await?;
        Ok(match t.get_root_hash() {
            Some(h) => to_hex(h),
            None => "0".repeat(64), 
        })
    }

    async fn build_local_merkle_snapshot(
        &self,
        prefix: &str,
    ) -> Result<(MerkleTree, HashMap<String, String>)> {
        let mut t = MerkleTree::new();
        let mut map = HashMap::new();

        let store = self.store.lock().await;
        let keys = store.scan(prefix); 
        for k in keys {
            if let Some(v) = store.get(&k) {
                t.insert(&k, &v);
                map.insert(k, v);
            }
        }
        Ok((t, map))
    }
    /// Reconcile a prefix by SCAN + GET from remote, then apply to local store.
    async fn reconcile_leaf(&self, addr: &str, prefix: &str) -> Result<()> {
        info!("RECONCILE prefix={:?}", prefix);

        let remote_keys = self.remote_scan_keys(addr, prefix).await?;

        if remote_keys.len() > self.leaf_threshold {
            log::warn!(
                "prefix {:?} has {} keys (> leaf_threshold {})",
                prefix,
                remote_keys.len(),
                self.leaf_threshold
            );
        }

        let mut remote_map: HashMap<String, Option<String>> = HashMap::new();
        for k in &remote_keys {
            remote_map.insert(k.clone(), self.remote_get(addr, k).await?);
        }

        let mut store = self.store.lock().await;

        for (k, maybe_v) in &remote_map {
            match maybe_v {
                Some(v) => {
                    let _ = store.set(k.clone(), v.clone());
                }
                None => {
                    let _ = store.delete(k);
                }
            }
        }

        let local_keys = store.scan(prefix);
        let remote_set: HashSet<&String> = remote_keys.iter().collect();
        for lk in local_keys {
            if !remote_set.contains(&lk) {
                let _ = store.delete(&lk);
            }
        }

        Ok(())
    }

    // ─────────────────── WIRE I/O (REMOTE) ───────────────────

    async fn remote_hash_hex(&self, addr: &str, prefix: &str) -> Result<String> {
        let cmd = if prefix.is_empty() {
            "HASH\r\n".to_string()
        } else {
            format!("HASH {prefix}\r\n")
        };
        let line = self.send_and_read_line(addr, &cmd).await?;
      
        let parts: Vec<&str> = line.trim_end().split_whitespace().collect();
        if parts.is_empty() || parts[0] != "HASH" {
            return Err(anyhow!("unexpected HASH response: {}", line.trim_end()));
        }
       
        let hex = parts.last().unwrap().to_string();
        if hex.len() != 64 {
            return Err(anyhow!("bad HASH hex length: {}", hex));
        }
        Ok(hex.to_lowercase())
    }

   
    async fn remote_scan_keys(&self, addr: &str, prefix: &str) -> Result<Vec<String>> {
        let cmd = format!("SCAN {prefix}\r\n");
        debug!("→ {} : {}", addr, cmd.trim_end());
        let mut stream = TcpStream::connect(addr)
            .await
            .with_context(|| format!("connect {}", addr))?;
        stream
            .write_all(cmd.as_bytes())
            .await
            .context("write SCAN")?;

        let mut reader = BufReader::new(stream);

        // header
        let mut header = String::new();
        let n = reader.read_line(&mut header).await?;
        if n == 0 {
            return Err(anyhow!("peer closed while reading SCAN header"));
        }
        let header = header.trim_end();
        let mut it = header.split_whitespace();
        if it.next() != Some("KEYS") {
            return Err(anyhow!("unexpected SCAN response: {}", header));
        }
        let count: usize = it
            .next()
            .ok_or_else(|| anyhow!("missing count after KEYS"))?
            .parse()
            .context("invalid count after KEYS")?;

        // keys
        let mut keys = Vec::with_capacity(count);
        for _ in 0..count {
            let mut line = String::new();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(anyhow!("peer closed while reading key list"));
            }
            keys.push(line.trim_end().to_string());
        }
        Ok(keys)
    }

    /// GET key → "VALUE <val>" hoặc "NOT_FOUND"
    async fn remote_get(&self, addr: &str, key: &str) -> Result<Option<String>> {
        let cmd = format!("GET {key}\r\n");
        let line = self.send_and_read_line(addr, &cmd).await?;
        let line = line.trim_end();
        if line == "NOT_FOUND" {
            return Ok(None);
        }
        if let Some(rest) = line.strip_prefix("VALUE ") {
            return Ok(Some(rest.to_string()));
        }
        Err(anyhow!("unexpected GET response for {key}: {}", line))
    }

    async fn send_and_read_line(&self, addr: &str, cmd: &str) -> Result<String> {
        debug!("→ {} : {}", addr, cmd.trim_end());
        let mut stream = TcpStream::connect(addr)
            .await
            .with_context(|| format!("connect {}", addr))?;
        stream
            .write_all(cmd.as_bytes())
            .await
            .context("write cmd")?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(anyhow!("peer closed connection"));
        }
        debug!("← {} : {}", addr, line.trim_end());
        Ok(line)
    }
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
