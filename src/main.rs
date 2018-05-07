
extern crate clap;
use clap::App;
use std::thread;
use std::net::SocketAddr;
use std::io::{Read, Write};
extern crate base64;
extern crate hex;

fn main() {
    let matches = App::new("LilTcp")
            .version("1.0")
            .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
            .about("Lets you manually, interactively exchange bytes with a TcpStream.")
            .args_from_usage("-s, --server 'Bind and assume the roll of server until you accept the first connection.'
                             <ip> 'Sets the bind/connect addr'")
            .get_matches();
    let ip = matches.value_of("ip").unwrap();
    if let Ok(addr) = ip.parse::<SocketAddr>() {
        match matches.occurrences_of("server") {
            0 => client(addr),
            1 | _ => server(addr),
        }
    } else {
        println!(">> Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
    }
}


fn client(addr: SocketAddr) {
    println!(">> Connecting to {:?}", &addr);
    let x = std::net::TcpStream::connect(&addr).expect("Failed to connect");
    println!(">> Connected!");
    go(x);
}

fn server(addr: SocketAddr) {
    println!(">> Binding to {:?}", &addr);
    let x = std::net::TcpListener::bind(&addr).expect("failed to bind");
    println!(">> Listening...");
    let (sock, addr) = x.accept().unwrap();
    println!(">> Connection from {:?}!", addr);
    go(sock);
}

#[derive(Copy, Clone)]
enum Setting {
    Hex, Utf8, Base64
}

static mut SHOW_HEX: bool = true;
static mut SHOW_STR: bool = true;
static mut SHOW_B64: bool = true;

fn go(mut stream: std::net::TcpStream) {
    stream.set_nodelay(true).unwrap();
    let mut stream2 = stream.try_clone().unwrap();
    stream2.set_nodelay(true).unwrap();
    println!(">> Enter just `/?` for a print of what commands exist");

    thread::spawn(move || {
        //OUTGOING thread
        let mut key_buffer = String::new();
        let mut setting = Setting::Utf8;
        loop {
            key_buffer.clear();
            let mut in_bytes = std::io::stdin().read_line(&mut key_buffer).expect("reader died");
            if key_buffer[..in_bytes].chars().last() == Some('\n') {
                in_bytes -= 1;
            }
            if key_buffer[..in_bytes].chars().last() == Some('\r') {
                in_bytes -= 1;
            }
            if &key_buffer[0..in_bytes] == "/?" {
                println!("//////////////");
                println!("Commands are of form: '/' + [flag] + [encoding], except for commands {{('/' + '?') and (`'!' + [string]`)}}");
                println!("flags are all strings in {{'$', '-', '+'}}.");
                println!("  '$' sets the input encoding to parse what you type.");
                println!("  '-' removes the given encoding from the output set.");
                println!("  '+' adds the given encoding from the output set.");
                println!("encodings are strings in {{str, hex, b64}}\n  which map to utf-8, hexadecimal and base 64 respectively.");
                println!("excape the leading '/' with another '/' if you wish to send a string with '/' in index 0.");
                println!("The ('!' + [string]) command shows the encoding-translations for your own input without sending it");
                println!("////////////////\n");
            } else if key_buffer.chars().nth(0) == Some('!') {
                print_out(& key_buffer[1..in_bytes].as_bytes());
            }else if key_buffer.chars().nth(0) == Some('/') {
                if key_buffer[0..in_bytes].chars().nth(1) == Some('/') {
                    send_text(&key_buffer[1..in_bytes], &mut stream2, setting);
                } else {
                    //command 
                    match key_buffer[0..in_bytes].chars().nth(1) {
                        Some('-') => {
                            match &key_buffer[2..in_bytes] {
                                "str" => {
                                    unsafe { SHOW_STR = false; }
                                    println!(">> disabling utf-8 printing");
                                },
                                "hex" => {
                                    unsafe { SHOW_HEX = false; }
                                    println!(">> disabling hexadecimal printing");
                                },
                                "b64" => {
                                    unsafe { SHOW_B64 = false; }
                                    println!(">> disabling base 64 printing");
                                },
                                _ => prompt_help(),
                            }
                        },
                        Some('+') => {
                            match &key_buffer[2..in_bytes] {
                                "str" => {
                                    unsafe { SHOW_STR = true; }
                                    println!(">> enabling utf-8 printing");
                                },
                                "hex" => {
                                    unsafe { SHOW_HEX = true; }
                                    println!(">> enabling hexadecimal printing");
                                },
                                "b64" => {
                                    unsafe { SHOW_B64 = true; }
                                    println!(">> enabling base 64 printing");
                                },
                                _ => prompt_help(),
                            }
                        },
                        Some('$') => {
                            match &key_buffer[2..in_bytes] {
                                "str" => {
                                    setting = Setting::Utf8;
                                    println!("Parsing input as utf-8 string.");
                                },
                                "hex" => {
                                    setting = Setting::Hex;
                                    println!("Parsing input as hexadecimal string.");
                                },
                                "b64" => {
                                    setting = Setting::Base64;
                                    println!("Parsing input as base 64 string.");
                                },
                                _ => prompt_help(),
                            }
                        },
                        _ => prompt_help(),
                    }
                    continue;
                }
            } else {
                send_text(&key_buffer[..in_bytes], &mut stream2, setting);
            }
        }
    });

    let mut buffer = [0u8; 80];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => print_out(&buffer[..bytes_read]),
            Err(e) => {
                println!("{:?}", e);
                return;
            }
        }
    }
}

fn print_out(bytes: &[u8]) {
    unsafe {
        if SHOW_STR {
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[str] `{}`", s);
            }
        }
        if SHOW_B64 {
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[b64]  {}", base64::encode(s));
            }
        }
        if SHOW_HEX{
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[hex]  {}", hex::encode(s));
            }
        }
        println!();
    }
}

fn prompt_help() {
    println!(">> Malformed command. Enter `/?` for help");
}

fn send_text(text: &str, stream: &mut std::net::TcpStream, setting: Setting) {
    match setting {
        Setting::Utf8 => {
            stream.write(&text.as_bytes()).expect("writing went bad");
        },
        Setting::Base64 => {
            if let Ok(b) = base64::decode(&text) {
                stream.write(&b).expect("writing went bad");
            } else {
                println!(">> Failed to encode as b64");
            }
        },
        Setting::Hex => {
            if let Ok(b) = hex::decode(&text) {
                stream.write(&b).expect("writing went bad");
            } else {
                println!(">> Failed to encode as b64");
            }
        },
    }
}