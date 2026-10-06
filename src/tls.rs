use std::net::IpAddr;

use axum_server::tls_rustls::RustlsConfig;

use crate::config::Config;

/// Loads the configured certificate files, or generates a self-signed certificate
/// when none are configured.
pub async fn rustls_config(config: &Config) -> Result<RustlsConfig, String> {
    if let Some(files) = &config.tls_files {
        return RustlsConfig::from_pem_file(&files.cert, &files.key)
            .await
            .map_err(|e| format!("Unable to load TLS files ({:?}, {:?}): {e}", files.cert, files.key));
    }

    let names = subject_alt_names(config);
    tracing::warn!("No TLS files configured, using a generated self-signed certificate for {names:?}");
    let generated = rcgen::generate_simple_self_signed(names)
        .map_err(|e| format!("Unable to generate TLS certificate: {e}"))?;
    RustlsConfig::from_pem(
        generated.cert.pem().into_bytes(),
        generated.signing_key.serialize_pem().into_bytes(),
    )
    .await
    .map_err(|e| format!("Unable to use generated TLS certificate: {e}"))
}

/// Names the certificate is valid for: localhost, the bind address, and the public host in `REPLACED_URL`.
fn subject_alt_names(config: &Config) -> Vec<String> {
    let mut names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let bind_ip = config.address.ip();
    if !bind_ip.is_unspecified() {
        names.push(bind_ip.to_string());
    }
    names.push(public_host(&config.replaced_url).to_string());
    names.sort();
    names.dedup();
    names
}

/// Strips the port from `host:port`, `[v6]:port`, or a bare IPv6 address.
fn public_host(replaced_url: &str) -> &str {
    if let Some(rest) = replaced_url.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    if replaced_url.parse::<IpAddr>().is_ok() {
        return replaced_url;
    }
    replaced_url.split(':').next().unwrap_or(replaced_url)
}
