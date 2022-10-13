// Copyright 2019-2022 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use isahc::{AsyncBody, HttpClient};
use pbr::{ProgressBar, Units};
use pin_project_lite::pin_project;
use std::io::{Stdout, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncRead;
use tokio::io::{BufReader, ReadBuf};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};
use url::Url;

#[derive(Debug, Error)]
enum DownloadError {
    #[error("Cannot read a file header")]
    HeaderError,
}

pin_project! {
    /// Holds a Reader, tracks read progress and draw a progress bar.
    pub struct FetchProgress<R, W: Write> {
        #[pin]
        pub inner: R,
        pub progress_bar: ProgressBar<W>,
    }
}

impl<R, W: Write> FetchProgress<R, W> {
    pub fn finish(&mut self) {
        self.progress_bar.finish();
    }
}

impl<R: AsyncRead + Unpin, W: Write> AsyncRead for FetchProgress<R, W> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let prev_len = buf.filled().len();
        let r = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = r {
            self.progress_bar
                .add((buf.filled().len() - prev_len) as u64);
        }
        r
    }
}

impl FetchProgress<AsyncBody, Stdout> {
    pub async fn fetch_from_url(
        url: Url,
    ) -> anyhow::Result<FetchProgress<Compat<AsyncBody>, Stdout>> {
        let client = HttpClient::new()?;
        let total_size = {
            let resp = client.head(url.as_str())?;
            if resp.status().is_success() {
                resp.headers()
                    .get("content-length")
                    .and_then(|ct_len| ct_len.to_str().ok())
                    .and_then(|ct_len| ct_len.parse().ok())
                    .unwrap_or(0)
            } else {
                return Err(anyhow::anyhow!(DownloadError::HeaderError));
            }
        };

        let request = client.get_async(url.as_str()).await?;

        let mut pb = ProgressBar::new(total_size);
        pb.message("Downloading/Importing snapshot ");
        pb.set_units(Units::Bytes);
        pb.set_max_refresh_rate(Some(Duration::from_millis(500)));

        Ok(FetchProgress {
            progress_bar: pb,
            inner: request.into_body().compat(),
        })
    }
}

impl FetchProgress<BufReader<File>, Stdout> {
    pub async fn fetch_from_file(file: File) -> anyhow::Result<Self> {
        let total_size = file.metadata().await?.len();

        let mut pb = ProgressBar::new(total_size);
        pb.message("Importing snapshot ");
        pb.set_units(Units::Bytes);
        pb.set_max_refresh_rate(Some(Duration::from_millis(500)));

        Ok(FetchProgress {
            progress_bar: pb,
            inner: BufReader::new(file),
        })
    }
}