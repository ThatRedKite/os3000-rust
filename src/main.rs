use std::{env, process::exit, time::Duration, io};
use std::io::IoSliceMut;
use plotters::chart::MeshStyle;
use serialport::{available_ports, SerialPortBuilder, SerialPortType, SerialPortInfo, SerialPort};

use plotters::prelude::*;

const CR: u8 = 0x0D;
const S1: [u8; 3] = [0x53, 0x31, CR];

fn test_connection(mut port: Box<dyn SerialPort>) -> Result<(), u8> {
    let mut result_buf = [0u8];
    port.write(&S1).unwrap();
    port.flush().unwrap();
    port.read(&mut result_buf).unwrap();
    match result_buf[0] {
        0x41 => Ok(()),
        0x61 => Err(1),
        _ => Err(2)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("You need to specify a serial port!\nCommand usage: os3000-reader [PORT] (BAUD RATE)");
        exit(255);
    }
    let port_name = &args[1];
    let baud_rate:u32 = *&args.get(2).unwrap_or(&(String::from("9600"))).parse().unwrap_or(9600);

    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(900))
        .data_bits(serialport::DataBits::Eight)
        .stop_bits(serialport::StopBits::Two)
        .parity(serialport::Parity::None)
        .flow_control(serialport::FlowControl::Software)
        .open();

    match port {
        Ok(mut port) => {
            //port.write_request_to_send(true).unwrap();
            let mut header = [0u8; 15];
            let mut data = [0u8; 1000];

            let bufs: &mut [IoSliceMut<'_>] = &mut [
                IoSliceMut::new(&mut header),
                IoSliceMut::new(&mut data)
            ];

            let mut s1_response = [0u8;2];
            port.write_data_terminal_ready(true).unwrap();
            port.write_request_to_send(true).unwrap();
            port.write(&S1).unwrap();
            port.flush().unwrap();
            port.read(&mut s1_response).unwrap();
            println!("{:X?}", s1_response);
            
            if s1_response[0] == 0x41 {
                loop {
                    std::thread::sleep(Duration::from_millis(500));
                    let ro = format!("R{}({},{},B)\r","1","0000","1000");
                    println!("{:X?}", ro.as_bytes());
                    port.write(ro.as_bytes()).unwrap();
                    port.flush().unwrap();
                    while port.bytes_to_read().unwrap() < 1000 {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    match port.read_vectored(bufs) {
                        Ok(a) => println!("{} {}", a, String::from_utf8_lossy(&data[0..16])),
                        Err(e) => match e.kind() {
                            io::ErrorKind::TimedOut => {eprintln!("e {} {}", port.bytes_to_read().unwrap() , String::from_utf8_lossy(&header).to_ascii_lowercase());},
                            _ => ()
                        }
                    }
                println!("Waveform received.");
                let root = SVGBackend::new("plot.svg", (4000,4000)).into_drawing_area();
                root.fill(&BLACK).unwrap();
                //println!("{:?}", data);
                let mut chart = ChartBuilder::on(&root)
                    .caption("Plot1", ("sans-serif", 30))
                    .set_label_area_size(LabelAreaPosition::Left, 50)
                    .set_label_area_size(LabelAreaPosition::Bottom, 50)
                    .build_cartesian_2d(0i32..1000, 0i32..255)
                    .unwrap()
                ;

                chart
                    .configure_mesh()
                    .bold_line_style(&plotters::style::full_palette::GREY_50)
                    .light_line_style(&plotters::style::full_palette::GREY_200)
                    .x_labels(8)
                    .y_labels(8)
                    .draw()
                    .unwrap()
                ;
                chart.draw_series(LineSeries::new((0..1000).map(|x| (x as i32, 127i32)), *&full_palette::GREY_500.stroke_width(3))).unwrap();
                
                chart
                    .draw_series(LineSeries::new(data.iter().enumerate().map(|(x, y)| (x as i32, *y as i32)), *&GREEN.stroke_width(5))).unwrap();
                
                //println!("Hi!");
                root.present().unwrap();
                port.clear(serialport::ClearBuffer::Input).unwrap();
                }
            }

        }
        Err(e) => {
            eprintln!("Failed to open port {}. The cause was {}", port_name, e);
            exit(255);
        }
        
    }


}
