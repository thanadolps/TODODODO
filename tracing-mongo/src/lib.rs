use tokio::sync::mpsc;
use tracing::Subscriber;
use tracing_subscriber::{filter::FilterFn, registry::LookupSpan, Layer};

use mongodb::{bson::Document, Client, Collection};

#[derive(Clone)]
pub struct MongoLogger {
    tx: mpsc::UnboundedSender<Document>,
}

async fn writer_task(mut rx: mpsc::UnboundedReceiver<Document>, collection: Collection<Document>) {
    while let Some(doc) = rx.recv().await {
        if let Err(err) = collection.insert_one(&doc, None).await {
            tracing::warn!(target: "tracing-mongo", error = ?err, doc=?doc, "Failed to write log to mongodb");
        }
    }
}

impl MongoLogger {
    pub async fn new(uri: &str, db: &str, collection: &str) -> mongodb::error::Result<Self> {
        let client = &Client::with_uri_str(uri).await?;
        let db = client.database(db);
        let collection = db.collection::<Document>(collection);

        // Spawn the writer task, and channel the document to it
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let collection = collection.to_owned();
            tokio::spawn(writer_task(rx, collection))
        };

        Ok(MongoLogger { tx })
    }

    /// Return [Layer] which write logs to mongodb in JSON format.
    pub fn layer<S>(self) -> impl tracing_subscriber::Layer<S>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        tracing_subscriber::fmt::layer()
            .json()
            .with_writer(move || self.clone())
            .with_filter(FilterFn::new(|meta| meta.target() != "tracing-mongo"))
    }
}

impl std::io::Write for MongoLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use std::io::{Error, ErrorKind};

        // Convert tracing's json log to mongo's document
        let doc: Document = serde_json::from_slice(buf).map_err(|err| {
            tracing::warn!(target: "tracing-mongo", error = ?err, "Failed to convert logging data to BSON document");
            Error::new(ErrorKind::InvalidData, err)
        })?;

        // Send the document to writer task
        self.tx.send(doc).map_err(|err| {
            tracing::warn!(target: "tracing-mongo", error = ?err, "Failed to send log to writer task");
            Error::new(ErrorKind::Other, err)
        })?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
