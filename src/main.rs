use std::{
    io::{self, Write},
    process::exit,
    thread,
    time::Duration,
};

use console::{style, Term};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let term = Term::stdout();

    println!("parsed command: {:#04x?}", parse_write("write 1 2 567")?);

    let ports = serialport::available_ports().context("No ports found!")?;

    let arcane_port = ports
        .iter()
        .find(|p| match &p.port_type {
            serialport::SerialPortType::UsbPort(usb_info) => {
                usb_info.product.as_deref() == Some("ARCANE Hub")
            }
            _ => false,
        })
        .context("No hub found")?;

    let mut port = serialport::new(&arcane_port.port_name, 115_200)
        .timeout(Duration::from_millis(10))
        .open()
        .context("Failed to open serial port")?;

    // term.clear_screen()?;
    term.set_title("ARCANE CLI");
    println!("{}", style("Welcome to the ARCANE console!").bold().cyan());
    println!("enter \"help\" to see your options\n");

    loop {
        println!("Please enter an ARCANE configuration command");

        print!("▶ ");
        io::stdout().flush()?;

        let mut command = String::new();
        io::stdin()
            .read_line(&mut command)
            .context("Can't read from stdin")?;

        match command.as_str().trim_end() {
            "help" => {
                println!("\navailable commands:\n");
                println!("write <node_id> <param_index> <param_value>");
                println!("read  <node_id> <param_index> <param_value>");
            }
            cmd if cmd.starts_with("read") => println!("read not implemented yet!"),
            cmd if cmd.starts_with("write") => match parse_write(&command) {
                Ok(data) => {
                    port.write(&data).context("Write failed!")?;
                }
                Err(e) => println!("Error: {}", e),
            },
            _ => println!("unknown command..."),
        }

        let output = "This is a test".as_bytes();
        port.write(output).context("Write failed!")?;

        thread::sleep(Duration::from_millis(20));

        let mut serial_buf: Vec<u8> = vec![0; 32];
        port.read(serial_buf.as_mut_slice())
            .context("Found no data!")
            .ok();

        thread::sleep(Duration::from_millis(20));
    }
}

/// Parses a command string into a byte array suitable for serial communication.
///
/// # Arguments
///
/// * `command` - A string slice containing the command to be parsed.
///
/// # Returns
///
/// A `Result` containing a `Vec<u8>` if the command is successfully parsed, or an error message if the parsing fails.
///
/// The ARCANE hub expects an 11 byte long array: [function_code, node_id, parameter_index, parameter_length, ...parameter_data, ...padding zeros]
///
/// # Example
///
/// ```
/// let command = "write 1 1 567";
/// let result = parse_write(command).unwrap();
/// assert_eq!(result, vec![0x01, 0x01, 0x01, 0x02, 0x02, 0x37, 0x00, 0x00, 0x00, 0x00, 0x00]);
/// ```
fn parse_write(command: &str) -> Result<Vec<u8>> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() == 4 {
        if let (Ok(node_id), Ok(param_index), Ok(param_value)) = (
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
            parts[3].parse::<u64>(),
        ) {
            // 0x07 corresponds to CFGW message
            let mut data = vec![0x07, node_id, param_index];

            let mut value_bytes = param_value.to_be_bytes().to_vec();
            value_bytes.retain(|&x| x != 0); // Remove leading zeros

            let payload_length = value_bytes.len() as u8;
            if payload_length > 7 {
                return Err(anyhow::anyhow!("Payload exceeds maximum length of 7 bytes"));
            }

            data.push(payload_length); // Add payload length

            data.extend_from_slice(&value_bytes); // Add actual payload bytes

            while data.len() < 11 {
                // Ensure total length is 11 (3 for initial bytes + 1 for length + 7 for payload)
                data.push(0x00); // Pad with zeros
            }

            return Ok(data);
        } else {
            return Err(anyhow::anyhow!(
                "Invalid node_id, param_index, or param_value"
            ));
        }
    } else {
        return Err(anyhow::anyhow!("Invalid command format"));
    }
}
