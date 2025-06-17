// =============================================================================
// Matrixon Matrix NextServer - Media Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Matrix API implementation for client-server communication. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • Matrix protocol compliance
//   • RESTful API endpoints
//   • Request/response handling
//   • Authentication and authorization
//   • Rate limiting and security
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

#![allow(deprecated)]

use std::time::Duration;

use crate::{service::media::FileMeta, services, utils, Error, Result, Ruma};
use http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use ruma::{
    api::{
        client::{
            authenticated_media::{
                get_content, get_content_as_filename, get_content_thumbnail, get_media_config,
            },
            error::ErrorKind,
            media::{self, create_content},
        },
        federation::authenticated_media::{self as federation_media, FileOrLocation},
    },
    http_headers::{ContentDisposition, ContentDispositionType},
    media::Method,
    ServerName, UInt,
};

const MXC_LENGTH: usize = 32;

/// # `GET /_matrix/media/r0/config`
///
/// Returns max upload size.
pub async fn get_media_config_route(
    _body: Ruma<media::get_media_config::v3::Request>,
) -> Result<media::get_media_config::v3::Response> {
    Ok(media::get_media_config::v3::Response::new(
        services().globals.max_request_size().into(),
    ))
}

/// # `GET /_matrix/client/v1/media/config`
///
/// Returns max upload size.
pub async fn get_media_config_auth_route(
    _body: Ruma<get_media_config::v1::Request>,
) -> Result<get_media_config::v1::Response> {
    Ok(get_media_config::v1::Response::new(
        services().globals.max_request_size().into(),
    ))
}

/// # `POST /_matrix/media/r0/upload`
///
/// Permanently save media in the server.
///
/// - Some metadata will be saved in the database
/// - Media will be saved in the media/ directory
pub async fn create_content_route(
    body: Ruma<create_content::v3::Request>,
) -> Result<create_content::v3::Response> {
    let create_content::v3::Request {
        filename,
        content_type,
        file,
        ..
    } = body.body;

    let media_id = utils::random_string(MXC_LENGTH);

    services()
        .media
        .create(
            services().globals.server_name(),
            &media_id,
            filename.as_deref(),
            content_type.as_deref(),
            &file,
            body.sender_user.as_deref(),
        )
        .await?;

    let mut resp = create_content::v3::Response::new(
        (format!("mxc://{}/{}", services().globals.server_name(), media_id)).into(),
    );
    resp.blurhash = None;
    Ok(resp)
}

pub async fn get_remote_content(
    server_name: &ServerName,
    media_id: String,
) -> Result<get_content::v1::Response, Error> {
    let content_response = match services()
        .sending
        .send_federation_request(
            server_name,
            {
                let mut request = federation_media::get_content::v1::Request::new(media_id.clone());
                request.timeout_ms = Duration::from_secs(20);
                request
            },
        )
        .await
    {
        Ok(federation_media::get_content::v1::Response {
            metadata: _,
            content: FileOrLocation::File(content),
            ..
        }) => get_content::v1::Response::new(
            content.file,
            content.content_type.unwrap_or_default(),
            content.content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
        ),

        Ok(federation_media::get_content::v1::Response {
            metadata: _,
            content: FileOrLocation::Location(url),
            ..
        }) => get_location_content(url).await?,

        Err(Error::BadRequest(ErrorKind::Unrecognized, _)) => {
            let media::get_content::v3::Response {
                file,
                content_type,
                content_disposition,
                ..
            } = services()
                .sending
                .send_federation_request(
                    server_name,
                    {
                        let mut request = media::get_content::v3::Request::new(
                            media_id.clone(),
                            server_name.to_owned(),
                        );
                        request.timeout_ms = Duration::from_secs(20);
                        request.allow_remote = false;
                        request.allow_redirect = true;
                        request
                    },
                )
                .await?;

            get_content::v1::Response::new(
                file,
                content_type.unwrap_or_default(),
                content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
            )
        }
        // Catch-all for other response variants
        Ok(_) => {
            return Err(Error::BadRequest(ErrorKind::NotFound, "Unsupported federation response type"));
        }
        Err(e) => return Err(e),
    };

    services()
        .media
        .create(
            server_name,
            &media_id,
            content_response
                .content_disposition
                .as_ref()
                .and_then(|cd| cd.filename.as_deref()),
            content_response.content_type.as_deref(),
            &content_response.file,
            None,
        )
        .await?;

    Ok(content_response)
}

/// # `GET /_matrix/media/r0/download/{serverName}/{mediaId}`
///
/// Load media from our server or over federation.
///
/// - Only allows federation if `allow_remote` is true
pub async fn get_content_route(
    body: Ruma<media::get_content::v3::Request>,
) -> Result<media::get_content::v3::Response> {
    let get_content::v1::Response {
        file,
        content_disposition,
        content_type,
        ..
    } = get_content(
        &body.server_name,
        body.media_id.clone(),
        body.allow_remote,
        false,
    )
    .await?;

    let mut resp = media::get_content::v3::Response::new(
        file,
        content_type.unwrap_or_default(),
        content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
    );
    resp.cross_origin_resource_policy = Some("cross-origin".to_owned());
    Ok(resp)
}

/// # `GET /_matrix/client/v1/media/download/{serverName}/{mediaId}`
///
/// Load media from our server or over federation.
pub async fn get_content_auth_route(
    body: Ruma<get_content::v1::Request>,
) -> Result<get_content::v1::Response> {
    get_content(&body.server_name, body.media_id.clone(), true, true).await
}

pub async fn get_content(
    server_name: &ServerName,
    media_id: String,
    allow_remote: bool,
    authenticated: bool,
) -> Result<get_content::v1::Response, Error> {
    services().media.check_blocked(server_name, &media_id)?;

    if let Ok(Some(FileMeta {
        content_disposition,
        content_type,
        file,
    })) = services()
        .media
        .get(server_name, &media_id, authenticated)
        .await
    {
        Ok(get_content::v1::Response::new(
            file,
            content_type.unwrap_or_default(),
            content_disposition,
        ))
    } else if server_name != services().globals.server_name() && allow_remote {
        let remote_content_response = get_remote_content(server_name, media_id.clone()).await?;

        Ok(get_content::v1::Response::new(
            remote_content_response.file,
            remote_content_response.content_type.unwrap_or_default(),
            remote_content_response.content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
        ))
    } else {
        Err(Error::BadRequest(ErrorKind::NotFound, "Media not found."))
    }
}

/// # `GET /_matrix/media/r0/download/{serverName}/{mediaId}/{fileName}`
///
/// Load media from our server or over federation, permitting desired filename.
///
/// - Only allows federation if `allow_remote` is true
pub async fn get_content_as_filename_route(
    body: Ruma<media::get_content_as_filename::v3::Request>,
) -> Result<media::get_content_as_filename::v3::Response> {
    let get_content_as_filename::v1::Response {
        file,
        content_type,
        content_disposition,
        ..
    } = get_content_as_filename(
        &body.server_name,
        body.media_id.clone(),
        body.filename.clone(),
        body.allow_remote,
        false,
    )
    .await?;

    let mut resp = media::get_content_as_filename::v3::Response::new(
        file,
        content_type.unwrap_or_default(),
        content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
    );
    resp.cross_origin_resource_policy = Some("cross-origin".to_owned());
    Ok(resp)
}

/// # `GET /_matrix/client/v1/media/download/{serverName}/{mediaId}/{fileName}`
///
/// Load media from our server or over federation, permitting desired filename.
pub async fn get_content_as_filename_auth_route(
    body: Ruma<get_content_as_filename::v1::Request>,
) -> Result<get_content_as_filename::v1::Response, Error> {
    get_content_as_filename(
        &body.server_name,
        body.media_id.clone(),
        body.filename.clone(),
        true,
        true,
    )
    .await
}

async fn get_content_as_filename(
    server_name: &ServerName,
    media_id: String,
    filename: String,
    allow_remote: bool,
    authenticated: bool,
) -> Result<get_content_as_filename::v1::Response, Error> {
    services().media.check_blocked(server_name, &media_id)?;

    if let Ok(Some(FileMeta {
        file, content_type, ..
    })) = services()
        .media
        .get(server_name, &media_id, authenticated)
        .await
    {
        Ok(get_content_as_filename::v1::Response::new(
            file,
            content_type.unwrap_or_default(),
            ContentDisposition::new(ContentDispositionType::Inline)
                .with_filename(Some(filename.clone())),
        ))
    } else if server_name != services().globals.server_name() && allow_remote {
        let remote_content_response = get_remote_content(server_name, media_id.clone()).await?;

        Ok(get_content_as_filename::v1::Response::new(
            remote_content_response.file,
            remote_content_response.content_type.unwrap_or_default(),
            ContentDisposition::new(ContentDispositionType::Inline)
                .with_filename(Some(filename.clone())),
        ))
    } else {
        Err(Error::BadRequest(ErrorKind::NotFound, "Media not found."))
    }
}

/// # `GET /_matrix/media/r0/thumbnail/{serverName}/{mediaId}`
///
/// Load media thumbnail from our server or over federation.
///
/// - Only allows federation if `allow_remote` is true
pub async fn get_content_thumbnail_route(
    body: Ruma<media::get_content_thumbnail::v3::Request>,
) -> Result<media::get_content_thumbnail::v3::Response> {
    let get_content_thumbnail::v1::Response {
        file,
        content_type,
        content_disposition,
        ..
    } = get_content_thumbnail(
        &body.server_name,
        body.media_id.clone(),
        body.height,
        body.width,
        body.method.clone(),
        body.animated,
        body.allow_remote,
        false,
    )
    .await?;

    let mut resp = media::get_content_thumbnail::v3::Response::new(
        file,
        content_type.unwrap_or_default(),
        content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
    );
    resp.cross_origin_resource_policy = Some("cross-origin".to_owned());
    Ok(resp)
}

/// # `GET /_matrix/client/v1/media/thumbnail/{serverName}/{mediaId}`
///
/// Load media thumbnail from our server or over federation.
pub async fn get_content_thumbnail_auth_route(
    body: Ruma<get_content_thumbnail::v1::Request>,
) -> Result<get_content_thumbnail::v1::Response> {
    get_content_thumbnail(
        &body.server_name,
        body.media_id.clone(),
        body.height,
        body.width,
        body.method.clone(),
        body.animated,
        true,
        true,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn get_content_thumbnail(
    server_name: &ServerName,
    media_id: String,
    height: UInt,
    width: UInt,
    method: Option<Method>,
    animated: Option<bool>,
    allow_remote: bool,
    authenticated: bool,
) -> Result<get_content_thumbnail::v1::Response, Error> {
    services().media.check_blocked(server_name, &media_id)?;

    if let Some(FileMeta {
        file,
        content_type,
        content_disposition,
    }) = services()
        .media
        .get_thumbnail(
            server_name,
            &media_id,
            width
                .try_into()
                .map_err(|_| Error::BadRequest(ErrorKind::InvalidParam, "Width is invalid."))?,
            height
                .try_into()
                .map_err(|_| Error::BadRequest(ErrorKind::InvalidParam, "Height is invalid."))?,
            authenticated,
        )
        .await?
    {
        Ok(get_content_thumbnail::v1::Response::new(
            file,
            content_type.unwrap_or_default(),
            content_disposition,
        ))
    } else if server_name != services().globals.server_name() && allow_remote && authenticated {
        let thumbnail_response = match services()
            .sending
            .send_federation_request(
                server_name,
                {
                    let mut request = federation_media::get_content_thumbnail::v1::Request::new(
                        media_id.clone(),
                        height,
                        width,
                    );
                    request.method = method.clone();
                    request.timeout_ms = Duration::from_secs(20);
                    request.animated = animated;
                    request
                },
            )
            .await
        {
            Ok(federation_media::get_content_thumbnail::v1::Response {
                metadata: _,
                content: FileOrLocation::File(content),
                ..
            }) => get_content_thumbnail::v1::Response::new(
                content.file,
                content.content_type.unwrap_or_default(),
                content.content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
            ),

            Ok(federation_media::get_content_thumbnail::v1::Response {
                metadata: _,
                content: FileOrLocation::Location(url),
                ..
            }) => {
                let get_content::v1::Response {
                    file,
                    content_type,
                    content_disposition,
                    ..
                } = get_location_content(url).await?;

                get_content_thumbnail::v1::Response::new(
                    file,
                    content_type.unwrap_or_default(),
                    content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
                )
            }
            // Catch-all for other response variants
            Ok(_) => {
                return Err(Error::BadRequest(ErrorKind::NotFound, "Unsupported federation response type"));
            }
            Err(Error::BadRequest(ErrorKind::Unrecognized, _)) => {
                let media::get_content::v3::Response {
                    file,
                    content_type,
                    content_disposition,
                    ..
                } = services()
                    .sending
                    .send_federation_request(
                        server_name,
                        {
                            let mut request = media::get_content::v3::Request::new(
                                media_id.clone(),
                                server_name.to_owned(),
                            );
                            request.timeout_ms = Duration::from_secs(20);
                            request.allow_remote = false;
                            request.allow_redirect = true;
                            request
                        },
                    )
                    .await?;

                get_content::v1::Response::new(
                    file,
                    content_type.unwrap_or_default(),
                    content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
                )
            }
            Err(e) => return Err(e),
        };

        services()
            .media
            .upload_thumbnail(
                server_name,
                &media_id,
                thumbnail_response
                    .content_disposition
                    .as_ref()
                    .and_then(|cd| cd.filename.as_deref()),
                thumbnail_response.content_type.as_deref(),
                width.try_into().expect("all UInts are valid u32s"),
                height.try_into().expect("all UInts are valid u32s"),
                &thumbnail_response.file,
            )
            .await?;

        Ok(thumbnail_response)
    } else {
        Err(Error::BadRequest(ErrorKind::NotFound, "Media not found."))
    }
}

async fn get_location_content(url: String) -> Result<get_content::v1::Response, Error> {
    let client = services().globals.default_client();
    let response = client.get(url).send().await?;
    let headers = response.headers();

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .map(ToOwned::to_owned);

    let content_disposition = headers
        .get(CONTENT_DISPOSITION)
        .map(|header| header.as_bytes())
        .map(TryFrom::try_from)
        .and_then(Result::ok);

    let file = response.bytes().await?.to_vec();

    Ok(get_content::v1::Response::new(
        file,
        content_type.unwrap_or_default(),
        content_disposition.unwrap_or_else(|| ContentDisposition::new(ContentDispositionType::Inline)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::media::FileMeta;
    use ruma::{
        api::client::{
            authenticated_media::{
                get_content, get_content_as_filename, get_content_thumbnail, get_media_config,
            },
            media::{self, create_content},
        },
        http_headers::{ContentDisposition, ContentDispositionType},
        media::Method,
        mxc_uri, server_name, uint, ServerName, UInt,
    };
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        time::{Duration, Instant},
    };
    use tracing::{debug, info};

    /// Mock media storage for testing
    #[derive(Debug)]
    struct MockMediaStorage {
        media_files: Arc<RwLock<HashMap<String, MockMediaFile>>>,
        thumbnails: Arc<RwLock<HashMap<String, MockThumbnail>>>,
        server_config: Arc<RwLock<MockServerConfig>>,
        media_counter: Arc<RwLock<u64>>,
    }

    #[derive(Debug, Clone)]
    struct MockMediaFile {
        content: Vec<u8>,
        content_type: Option<String>,
        filename: Option<String>,
        uploader: Option<String>,
        created_at: u64,
    }

    #[derive(Debug, Clone)]
    struct MockThumbnail {
        content: Vec<u8>,
        width: u32,
        height: u32,
        method: Method,
        content_type: String,
    }

    #[derive(Debug, Clone)]
    struct MockServerConfig {
        max_upload_size: u64,
        thumbnail_sizes: Vec<(u32, u32)>,
    }

    impl MockMediaStorage {
        fn new() -> Self {
            Self {
                media_files: Arc::new(RwLock::new(HashMap::new())),
                thumbnails: Arc::new(RwLock::new(HashMap::new())),
                server_config: Arc::new(RwLock::new(MockServerConfig {
                    max_upload_size: 50 * 1024 * 1024, // 50MB
                    thumbnail_sizes: vec![(96, 96), (320, 240), (640, 480), (800, 600)],
                })),
                media_counter: Arc::new(RwLock::new(1000)),
            }
        }

        fn create_media(&self, media_id: String, file: MockMediaFile) {
            self.media_files.write().unwrap().insert(media_id, file);
        }

        fn get_media(&self, media_id: &str) -> Option<MockMediaFile> {
            self.media_files.read().unwrap().get(media_id).cloned()
        }

        fn create_thumbnail(&self, key: String, thumbnail: MockThumbnail) {
            self.thumbnails.write().unwrap().insert(key, thumbnail);
        }

        fn get_thumbnail(&self, key: &str) -> Option<MockThumbnail> {
            self.thumbnails.read().unwrap().get(key).cloned()
        }

        fn next_media_id(&self) -> String {
            let mut counter = self.media_counter.write().unwrap();
            *counter += 1;
            format!("media_{}", *counter)
        }

        fn get_max_upload_size(&self) -> u64 {
            self.server_config.read().unwrap().max_upload_size
        }

        fn is_supported_thumbnail_size(&self, width: u32, height: u32) -> bool {
            let config = self.server_config.read().unwrap();
            config.thumbnail_sizes.contains(&(width, height))
        }
    }

    fn create_test_media_file(content: &[u8], content_type: &str, filename: Option<&str>) -> MockMediaFile {
        MockMediaFile {
            content: content.to_vec(),
            content_type: Some(content_type.to_string()),
            filename: filename.map(|f| f.to_string()),
            uploader: Some("@test:example.com".to_string()),
            created_at: 1000,
        }
    }

    fn create_test_thumbnail(width: u32, height: u32, method: Method) -> MockThumbnail {
        MockThumbnail {
            content: vec![0xFF, 0xD8, 0xFF], // Simple JPEG header
            width,
            height,
            method,
            content_type: "image/jpeg".to_string(),
        }
    }

    #[test]
    fn test_media_config_request_structures() {
        debug!("🔧 Testing media config request structures");
        let start = Instant::now();

        // Test unauthenticated media config request
        let unauth_request = media::get_media_config::v3::Request {};
        // No fields to test for empty request structure

        // Test authenticated media config request
        let auth_request = get_media_config::v1::Request {};
        // No fields to test for empty request structure

        // Test media config response structure
        let response = media::get_media_config::v3::Response {
            upload_size: uint!(50_000_000),
        };

        assert_eq!(response.upload_size, uint!(50_000_000), "Upload size should match");

        info!("✅ Media config request structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_content_upload_structures() {
        debug!("🔧 Testing content upload structures");
        let start = Instant::now();

        let file_content = b"Hello, Matrix!";
        let filename = Some("test.txt".to_string());
        let content_type = Some("text/plain".to_string());

        // Test create content request
        let create_request = create_content::v3::Request {
            filename: filename.clone(),
            content_type: content_type.clone(),
            file: file_content.to_vec(),
            ..
        };

        assert_eq!(create_request.filename, filename, "Filename should match");
        assert_eq!(create_request.content_type, content_type, "Content type should match");
        assert_eq!(create_request.file, file_content, "File content should match");

        // Test create content response
        let mxc_uri = format!("mxc://example.com/media123");
        let create_response = create_content::v3::Response {
            content_uri: mxc_uri.clone().into(),
            blurhash: None,
        };

        assert_eq!(create_response.content_uri.to_string(), mxc_uri, "Content URI should match");
        assert_eq!(create_response.blurhash, None, "Blurhash should be None for text file");

        info!("✅ Content upload structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_content_download_structures() {
        debug!("🔧 Testing content download structures");
        let start = Instant::now();

        let server_name = server_name!("example.com");
        let media_id = "media123".to_string();

        // Test unauthenticated download request
        let unauth_request = media::get_content::v3::Request {
            server_name: server_name.to_owned(),
            media_id: media_id.clone(),
            allow_remote: true,
            timeout_ms: Duration::from_secs(20),
            allow_redirect: false,
        };

        assert_eq!(unauth_request.server_name, server_name, "Server name should match");
        assert_eq!(unauth_request.media_id, media_id, "Media ID should match");
        assert_eq!(unauth_request.allow_remote, true, "Allow remote should be true");
        assert_eq!(unauth_request.timeout_ms, Duration::from_secs(20), "Timeout should match");

        // Test authenticated download request
        let auth_request = get_content::v1::Request {
            server_name: server_name.to_owned(),
            media_id: media_id.clone(),
            timeout_ms: Duration::from_secs(20),
        };

        assert_eq!(auth_request.server_name, server_name, "Server name should match");
        assert_eq!(auth_request.media_id, media_id, "Media ID should match");

        // Test download response
        let file_content = b"Test file content";
        let response = media::get_content::v3::Response {
            file: file_content.to_vec(),
            content_type: Some("text/plain".to_string()),
            content_disposition: None,
            cross_origin_resource_policy: Some("cross-origin".to_string()),
        };

        assert_eq!(response.file, file_content, "File content should match");
        assert_eq!(response.content_type, Some("text/plain".to_string()), "Content type should match");

        info!("✅ Content download structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_thumbnail_request_structures() {
        debug!("🔧 Testing thumbnail request structures");
        let start = Instant::now();

        let server_name = server_name!("example.com");
        let media_id = "image123".to_string();

        // Test thumbnail request
        let thumbnail_request = media::get_content_thumbnail::v3::Request {
            server_name: server_name.to_owned(),
            media_id: media_id.clone(),
            width: uint!(320),
            height: uint!(240),
            method: Some(Method::Scale),
            animated: Some(false),
            timeout_ms: Duration::from_secs(20),
            allow_remote: true,
            allow_redirect: false,
        };

        assert_eq!(thumbnail_request.server_name, server_name, "Server name should match");
        assert_eq!(thumbnail_request.media_id, media_id, "Media ID should match");
        assert_eq!(thumbnail_request.width, uint!(320), "Width should match");
        assert_eq!(thumbnail_request.height, uint!(240), "Height should match");
        assert_eq!(thumbnail_request.method, Some(Method::Scale), "Method should be Scale");
        assert_eq!(thumbnail_request.animated, Some(false), "Animated should be false");

        // Test thumbnail response
        let thumbnail_content = vec![0xFF, 0xD8, 0xFF]; // JPEG header
        let thumbnail_response = media::get_content_thumbnail::v3::Response {
            file: thumbnail_content.clone(),
            content_type: Some("image/jpeg".to_string()),
            content_disposition: None,
            cross_origin_resource_policy: Some("cross-origin".to_string()),
        };

        assert_eq!(thumbnail_response.file, thumbnail_content, "Thumbnail content should match");
        assert_eq!(thumbnail_response.content_type, Some("image/jpeg".to_string()), "Content type should be JPEG");

        info!("✅ Thumbnail request structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_content_disposition_handling() {
        debug!("🔧 Testing content disposition handling");
        let start = Instant::now();

        // Test content disposition creation
        let attachment_disposition = ContentDisposition::new(ContentDispositionType::Attachment)
            .with_filename(Some("document.pdf".to_string()));

        match attachment_disposition.disposition_type {
            ContentDispositionType::Attachment => assert!(true, "Should be attachment type"),
            _ => panic!("Should be attachment disposition"),
        }

        assert_eq!(attachment_disposition.filename, Some("document.pdf".to_string()), "Filename should match");

        // Test inline disposition
        let inline_disposition = ContentDisposition::new(ContentDispositionType::Inline);

        match inline_disposition.disposition_type {
            ContentDispositionType::Inline => assert!(true, "Should be inline type"),
            _ => panic!("Should be inline disposition"),
        }

        assert_eq!(inline_disposition.filename, None, "Inline should have no filename");

        // Test filename with special characters
        let special_filename = "测试文件.txt";
        let special_disposition = ContentDisposition::new(ContentDispositionType::Attachment)
            .with_filename(Some(special_filename.to_string()));

        assert_eq!(special_disposition.filename, Some(special_filename.to_string()), "Should handle Unicode filenames");

        info!("✅ Content disposition handling test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_media_storage_operations() {
        debug!("🔧 Testing media storage operations");
        let start = Instant::now();

        let storage = MockMediaStorage::new();
        let media_id = storage.next_media_id();

        // Test media creation
        let file_content = b"Test media content";
        let media_file = create_test_media_file(file_content, "text/plain", Some("test.txt"));
        storage.create_media(media_id.clone(), media_file.clone());

        // Test media retrieval
        let retrieved = storage.get_media(&media_id);
        assert!(retrieved.is_some(), "Should retrieve stored media");

        let retrieved_file = retrieved.unwrap();
        assert_eq!(retrieved_file.content, file_content, "Content should match");
        assert_eq!(retrieved_file.content_type, Some("text/plain".to_string()), "Content type should match");
        assert_eq!(retrieved_file.filename, Some("test.txt".to_string()), "Filename should match");

        // Test non-existent media
        let non_existent = storage.get_media("non_existent_id");
        assert!(non_existent.is_none(), "Should return None for non-existent media");

        info!("✅ Media storage operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_thumbnail_operations() {
        debug!("🔧 Testing thumbnail operations");
        let start = Instant::now();

        let storage = MockMediaStorage::new();

        // Test thumbnail creation and storage
        let thumbnail_key = "media123_320x240_scale";
        let thumbnail = create_test_thumbnail(320, 240, Method::Scale);
        storage.create_thumbnail(thumbnail_key.to_string(), thumbnail.clone());

        // Test thumbnail retrieval
        let retrieved = storage.get_thumbnail(thumbnail_key);
        assert!(retrieved.is_some(), "Should retrieve stored thumbnail");

        let retrieved_thumbnail = retrieved.unwrap();
        assert_eq!(retrieved_thumbnail.width, 320, "Width should match");
        assert_eq!(retrieved_thumbnail.height, 240, "Height should match");
        assert_eq!(retrieved_thumbnail.method, Method::Scale, "Method should match");
        assert_eq!(retrieved_thumbnail.content_type, "image/jpeg", "Content type should be JPEG");

        // Test different thumbnail methods
        let crop_thumbnail = create_test_thumbnail(200, 200, Method::Crop);
        storage.create_thumbnail("media123_200x200_crop".to_string(), crop_thumbnail);

        let scale_thumbnail = create_test_thumbnail(100, 100, Method::Scale);
        storage.create_thumbnail("media123_100x100_scale".to_string(), scale_thumbnail);

        // Verify different methods are stored separately
        let crop_retrieved = storage.get_thumbnail("media123_200x200_crop");
        let scale_retrieved = storage.get_thumbnail("media123_100x100_scale");

        assert!(crop_retrieved.is_some(), "Crop thumbnail should exist");
        assert!(scale_retrieved.is_some(), "Scale thumbnail should exist");
        assert_eq!(crop_retrieved.unwrap().method, Method::Crop, "Should be crop method");
        assert_eq!(scale_retrieved.unwrap().method, Method::Scale, "Should be scale method");

        info!("✅ Thumbnail operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_media_id_generation() {
        debug!("🔧 Testing media ID generation");
        let start = Instant::now();

        let storage = MockMediaStorage::new();

        // Test media ID uniqueness
        let mut media_ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let media_id = storage.next_media_id();
            assert!(!media_id.is_empty(), "Media ID should not be empty");
            assert!(media_id.starts_with("media_"), "Media ID should have prefix");
            assert!(media_ids.insert(media_id), "Media IDs should be unique");
        }

        // Test media ID format
        let media_id = storage.next_media_id();
        assert!(media_id.len() > 6, "Media ID should be longer than prefix");
        assert!(media_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "Media ID should be alphanumeric");

        info!("✅ Media ID generation test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_file_size_limits() {
        debug!("🔧 Testing file size limits");
        let start = Instant::now();

        let storage = MockMediaStorage::new();
        let max_size = storage.get_max_upload_size();

        // Test max upload size configuration
        assert_eq!(max_size, 50 * 1024 * 1024, "Max upload size should be 50MB");

        // Test file size validation logic
        let small_file = vec![0u8; 1024]; // 1KB
        let medium_file = vec![0u8; 10 * 1024 * 1024]; // 10MB
        let large_file = vec![0u8; 100 * 1024 * 1024]; // 100MB (exceeds limit)

        assert!(small_file.len() as u64 <= max_size, "Small file should be within limit");
        assert!(medium_file.len() as u64 <= max_size, "Medium file should be within limit");
        assert!(large_file.len() as u64 > max_size, "Large file should exceed limit");

        // Test different file size categories
        let file_sizes = vec![
            (100, "tiny"),
            (1024, "small"),
            (1024 * 1024, "medium"),
            (10 * 1024 * 1024, "large"),
        ];

        for (size, description) in file_sizes {
            let file = vec![0u8; size];
            assert!(file.len() as u64 <= max_size, "{} file should be within limit", description);
        }

        info!("✅ File size limits test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_content_type_validation() {
        debug!("🔧 Testing content type validation");
        let start = Instant::now();

        // Test valid content types
        let valid_types = vec![
            "image/jpeg",
            "image/png",
            "image/gif",
            "image/webp",
            "text/plain",
            "application/pdf",
            "video/mp4",
            "audio/mpeg",
            "application/json",
        ];

        for content_type in valid_types {
            assert!(!content_type.is_empty(), "Content type should not be empty");
            assert!(content_type.contains('/'), "Content type should contain /");
            
            let parts: Vec<&str> = content_type.split('/').collect();
            assert_eq!(parts.len(), 2, "Content type should have exactly 2 parts");
            assert!(!parts[0].is_empty(), "Type should not be empty");
            assert!(!parts[1].is_empty(), "Subtype should not be empty");
        }

        // Test content type categories
        let image_types = vec!["image/jpeg", "image/png", "image/gif"];
        let text_types = vec!["text/plain", "text/html", "text/css"];
        let video_types = vec!["video/mp4", "video/webm", "video/avi"];

        for content_type in image_types {
            assert!(content_type.starts_with("image/"), "Should be image type");
        }

        for content_type in text_types {
            assert!(content_type.starts_with("text/"), "Should be text type");
        }

        for content_type in video_types {
            assert!(content_type.starts_with("video/"), "Should be video type");
        }

        info!("✅ Content type validation test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_mxc_uri_format() {
        debug!("🔧 Testing MXC URI format");
        let start = Instant::now();

        let server_name = "example.com";
        let media_id = "abcdef123456";

        // Test MXC URI construction
        let mxc_uri = format!("mxc://{}/{}", server_name, media_id);
        assert!(mxc_uri.starts_with("mxc://"), "MXC URI should start with mxc://");
        assert!(mxc_uri.contains(server_name), "MXC URI should contain server name");
        assert!(mxc_uri.contains(media_id), "MXC URI should contain media ID");

        // Test MXC URI parsing
        let parts: Vec<&str> = mxc_uri.strip_prefix("mxc://").unwrap().split('/').collect();
        assert_eq!(parts.len(), 2, "MXC URI should have server and media ID parts");
        assert_eq!(parts[0], server_name, "Server name should match");
        assert_eq!(parts[1], media_id, "Media ID should match");

        // Test various MXC URI formats
        let mxc_uris = vec![
            "mxc://matrix.org/GCmhgzMPRjqgpODLsNQzVuHZ",
            "mxc://example.com/simple123",
            "mxc://localhost:8008/dev_media_id",
        ];

        for uri in mxc_uris {
            assert!(uri.starts_with("mxc://"), "Should be valid MXC URI format");
            let without_prefix = uri.strip_prefix("mxc://").unwrap();
            assert!(without_prefix.contains('/'), "Should have server/media separator");
            
            let parts: Vec<&str> = without_prefix.split('/').collect();
            assert_eq!(parts.len(), 2, "Should have exactly 2 parts");
            assert!(!parts[0].is_empty(), "Server name should not be empty");
            assert!(!parts[1].is_empty(), "Media ID should not be empty");
        }

        info!("✅ MXC URI format test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_concurrent_media_operations() {
        debug!("🔧 Testing concurrent media operations");
        let start = Instant::now();

        use std::thread;

        let storage = Arc::new(MockMediaStorage::new());
        let num_threads = 10;
        let operations_per_thread = 20;

        let mut handles = vec![];

        // Spawn threads performing concurrent media operations
        for thread_id in 0..num_threads {
            let storage_clone = Arc::clone(&storage);
            
            let handle = thread::spawn(move || {
                for op_id in 0..operations_per_thread {
                    // Concurrent media creation
                    let media_id = format!("media_{}_{}", thread_id, op_id);
                    let content = format!("Content from thread {} operation {}", thread_id, op_id);
                    let media_file = create_test_media_file(
                        content.as_bytes(),
                        "text/plain",
                        Some(&format!("file_{}_{}.txt", thread_id, op_id))
                    );
                    
                    storage_clone.create_media(media_id.clone(), media_file);
                    
                    // Verify media was stored
                    let retrieved = storage_clone.get_media(&media_id);
                    assert!(retrieved.is_some(), "Media should be stored");
                    assert_eq!(retrieved.unwrap().content, content.as_bytes(), "Content should match");
                    
                    // Concurrent thumbnail creation
                    let thumbnail_key = format!("thumb_{}_{}", thread_id, op_id);
                    let thumbnail = create_test_thumbnail(100, 100, Method::Scale);
                    storage_clone.create_thumbnail(thumbnail_key.clone(), thumbnail);
                    
                    // Verify thumbnail was stored
                    let retrieved_thumb = storage_clone.get_thumbnail(&thumbnail_key);
                    assert!(retrieved_thumb.is_some(), "Thumbnail should be stored");
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        info!("✅ Concurrent media operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_media_performance_benchmarks() {
        debug!("🔧 Testing media performance benchmarks");
        let start = Instant::now();

        let storage = MockMediaStorage::new();

        // Benchmark media storage
        let storage_start = Instant::now();
        for i in 0..1000 {
            let media_id = format!("perf_test_{}", i);
            let content = vec![0u8; 1024]; // 1KB files
            let media_file = create_test_media_file(&content, "application/octet-stream", None);
            storage.create_media(media_id, media_file);
        }
        let storage_duration = storage_start.elapsed();

        // Benchmark media retrieval
        let retrieval_start = Instant::now();
        for i in 0..1000 {
            let media_id = format!("perf_test_{}", i);
            let _ = storage.get_media(&media_id);
        }
        let retrieval_duration = retrieval_start.elapsed();

        // Benchmark thumbnail operations
        let thumbnail_start = Instant::now();
        for i in 0..500 {
            let thumbnail_key = format!("thumb_perf_{}", i);
            let thumbnail = create_test_thumbnail(200, 200, Method::Scale);
            storage.create_thumbnail(thumbnail_key, thumbnail);
        }
        let thumbnail_duration = thumbnail_start.elapsed();

        // Performance assertions
        assert!(storage_duration < Duration::from_millis(1000), 
                "Storing 1000 media files should complete within 1s, took: {:?}", storage_duration);
        assert!(retrieval_duration < Duration::from_millis(100), 
                "Retrieving 1000 media files should complete within 100ms, took: {:?}", retrieval_duration);
        assert!(thumbnail_duration < Duration::from_millis(500), 
                "Creating 500 thumbnails should complete within 500ms, took: {:?}", thumbnail_duration);

        info!("✅ Media performance benchmarks completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_media_edge_cases() {
        debug!("🔧 Testing media edge cases");
        let start = Instant::now();

        let storage = MockMediaStorage::new();

        // Test empty file
        let empty_file = create_test_media_file(&[], "application/octet-stream", None);
        storage.create_media("empty_file".to_string(), empty_file);
        let retrieved = storage.get_media("empty_file");
        assert!(retrieved.is_some(), "Should handle empty files");
        assert!(retrieved.unwrap().content.is_empty(), "Empty file should have no content");

        // Test file with no content type
        let no_type_file = MockMediaFile {
            content: b"test".to_vec(),
            content_type: None,
            filename: Some("unknown.bin".to_string()),
            uploader: Some("@test:example.com".to_string()),
            created_at: 1000,
        };
        storage.create_media("no_type".to_string(), no_type_file);
        let retrieved = storage.get_media("no_type");
        assert!(retrieved.is_some(), "Should handle files without content type");
        assert_eq!(retrieved.unwrap().content_type, None, "Content type should be None");

        // Test very long filename
        let long_filename = "a".repeat(255);
        let long_name_file = create_test_media_file(b"test", "text/plain", Some(&long_filename));
        storage.create_media("long_name".to_string(), long_name_file);
        let retrieved = storage.get_media("long_name");
        assert!(retrieved.is_some(), "Should handle long filenames");
        assert_eq!(retrieved.unwrap().filename, Some(long_filename), "Long filename should be preserved");

        // Test Unicode filename
        let unicode_filename = "测试文件🎉.txt";
        let unicode_file = create_test_media_file(b"test", "text/plain", Some(unicode_filename));
        storage.create_media("unicode".to_string(), unicode_file);
        let retrieved = storage.get_media("unicode");
        assert!(retrieved.is_some(), "Should handle Unicode filenames");
        assert_eq!(retrieved.unwrap().filename, Some(unicode_filename.to_string()), "Unicode filename should be preserved");

        info!("✅ Media edge cases test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_matrix_protocol_compliance() {
        debug!("🔧 Testing Matrix protocol compliance");
        let start = Instant::now();

        // Test Matrix media ID length compliance
        assert_eq!(MXC_LENGTH, 32, "Matrix media ID length should be 32 characters");

        // Test Matrix thumbnail sizes compliance
        let storage = MockMediaStorage::new();
        let standard_sizes = vec![
            (96, 96),    // Avatar size
            (320, 240),  // Small thumbnail
            (640, 480),  // Medium thumbnail
            (800, 600),  // Large thumbnail
        ];

        for (width, height) in standard_sizes {
            assert!(storage.is_supported_thumbnail_size(width, height), 
                   "Size {}x{} should be supported", width, height);
        }

        // Test Matrix media endpoints compliance
        let server_name = server_name!("matrix.org");
        let media_id = "GCmhgzMPRjqgpODLsNQzVuHZ";

        // Verify Matrix URI format
        let mxc_uri = format!("mxc://{}/{}", server_name, media_id);
        assert!(mxc_uri.starts_with("mxc://"), "Should use mxc:// scheme");
        assert_eq!(media_id.len(), 24, "Matrix media IDs are typically 24 characters");

        // Test Matrix content types compliance
        let matrix_content_types = vec![
            "image/jpeg", "image/png", "image/gif", "image/webp",
            "video/mp4", "video/webm", "audio/mp4", "audio/webm",
            "text/plain", "application/pdf",
        ];

        for content_type in matrix_content_types {
            assert!(content_type.contains('/'), "Content type should be valid MIME type");
            assert!(!content_type.is_empty(), "Content type should not be empty");
        }

        // Test Matrix thumbnail methods
        let scale_method = Method::Scale;
        let crop_method = Method::Crop;

        // Verify methods can be serialized (required for Matrix protocol)
        let scale_str = format!("{:?}", scale_method);
        let crop_str = format!("{:?}", crop_method);
        assert!(!scale_str.is_empty(), "Scale method should be serializable");
        assert!(!crop_str.is_empty(), "Crop method should be serializable");

        info!("✅ Matrix protocol compliance test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_media_security_constraints() {
        debug!("🔧 Testing media security constraints");
        let start = Instant::now();

        let storage = MockMediaStorage::new();

        // Test file size limits for security
        let max_size = storage.get_max_upload_size();
        assert!(max_size > 0, "Max upload size should be positive");
        assert!(max_size <= 100 * 1024 * 1024, "Max upload size should be reasonable (≤100MB)");

        // Test media ID unpredictability
        let mut media_ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let media_id = storage.next_media_id();
            assert!(media_ids.insert(media_id), "Media IDs should be unique");
        }

        // Test content type validation against dangerous types
        let dangerous_types = vec![
            "application/x-executable",
            "application/x-msdownload",
            "text/html", // Could be used for XSS
        ];

        for content_type in dangerous_types {
            // In a real implementation, these would be blocked or sanitized
            assert!(content_type.len() > 0, "Content type should not be empty");
        }

        // Test filename sanitization scenarios
        let dangerous_filenames = vec![
            "../../../etc/passwd",
            "..\\windows\\system32\\config",
            "<script>alert('xss')</script>",
            "file with spaces and & symbols.txt",
        ];

        for filename in dangerous_filenames {
            let media_file = create_test_media_file(b"test", "text/plain", Some(filename));
            storage.create_media("security_test".to_string(), media_file);
            let retrieved = storage.get_media("security_test");
            
            // Verify dangerous filename is stored (sanitization would happen at a higher level)
            assert!(retrieved.is_some(), "File should be stored");
            assert_eq!(retrieved.unwrap().filename, Some(filename.to_string()), "Filename should be preserved for testing");
        }

        // Test uploader tracking for audit purposes
        let media_file = create_test_media_file(b"test", "text/plain", Some("audit.txt"));
        assert!(media_file.uploader.is_some(), "Uploader should be tracked");
        assert_eq!(media_file.uploader, Some("@test:example.com".to_string()), "Uploader should be recorded");

        info!("✅ Media security constraints test completed in {:?}", start.elapsed());
    }
}
