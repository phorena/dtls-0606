use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use bytes::Bytes;
use clap::{App, AppSettings, Arg};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use util::conn::*;
use webrtc_dtls::config::ExtendedMasterSecretType;
use webrtc_dtls::Error;
use webrtc_dtls::{config::Config, crypto::Certificate, listener::listen};

// cargo run --example listen_selfsign -- --host 127.0.0.1:4444

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} [{}] {} - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                chrono::Local::now().format("%H:%M:%S.%6f"),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Trace)
        .init();

    let mut app = App::new("DTLS Server")
        .version("0.1.0")
        .author("Rain Liu <yliu@webrtc.rs>")
        .about("An example of DTLS Server")
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::SubcommandsNegateReqs)
        .arg(
            Arg::with_name("FULLHELP")
                .help("Prints more detailed help information")
                .long("fullhelp"),
        )
        .arg(
            Arg::with_name("host")
                .required_unless("FULLHELP")
                .takes_value(true)
                .default_value("127.0.0.1:4444")
                .long("host")
                .help("DTLS host name."),
        );

    let matches = app.clone().get_matches();

    if matches.is_present("FULLHELP") {
        app.print_long_help().unwrap();
        std::process::exit(0);
    }

    let host = matches.value_of("host").unwrap().to_owned();

    // Generate a certificate and private key to secure the connection
    let certificate = Certificate::generate_self_signed(vec!["localhost".to_owned()])?;

    let cfg = Config {
        certificates: vec![certificate],
        extended_master_secret: ExtendedMasterSecretType::Require,
        ..Default::default()
    };

    println!("listening {}...\ntype 'exit' to shutdown gracefully", host);

    let listener = Arc::new(listen(host, cfg).await?);

    // Simulate a chat session
    let (tx, rx): (
        Sender<(SocketAddr, Bytes, Arc<dyn Conn + Send + Sync>)>,
        Receiver<(SocketAddr, Bytes, Arc<dyn Conn + Send + Sync>)>,
    ) = unbounded();
    // let h = Arc::new(hub::Hub::new());
    let tx = Arc::new(tx);
    let h1 = Arc::new(hub::Hub2::new(tx));
    let rx2 = rx.clone();

    let listener2 = Arc::clone(&listener);
    let h2 = Arc::clone(&h1);
    let h3 = Arc::clone(&h1);
    tokio::spawn(async move {
        while let Ok((dtls_conn, _remote_addr)) = listener2.accept().await {
            // Register the connection with the chat hub
            h2.register(dtls_conn).await;
        }
    });

    tokio::spawn(async move {
        loop {
            match rx2.try_recv() {
                Ok((addr, data, _conn)) => {
                    println!("*******************{:?} {:?}", addr, data);
                    // listener.send(addr, data).await?;
                    _conn.send("hello".as_bytes()).await;
                    let conn2 = h3.get_conn(addr).await.unwrap();
                    let _result = conn2
                        .send("hello there************************".as_bytes())
                        .await;
                }
                Err(TryRecvError::Empty) => {
                    continue;
                }
                Err(why) => {
                    println!("&&&&&&&&&&&&&&&&&&{:?}", why);
                    break;
                }
            }
        }
    });

    h1.chat().await;

    Ok(listener.close().await?)

    /*
    let listener = Arc::new(listen(host, cfg).await?);

    // Simulate a chat session
    let h = Arc::new(hub::Hub::new());

    let listener2 = Arc::clone(&listener);
    let h2 = Arc::clone(&h);
    tokio::spawn(async move {
        while let Ok((dtls_conn, _remote_addr)) = listener2.accept().await {
            // Register the connection with the chat hub
            h2.register(dtls_conn).await;
        }
    });

    h.chat().await;

    Ok(listener.close().await?)
    */
}
