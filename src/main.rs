use rm_daemon_utils::itertools::Itertools;
use rm_daemon_utils::md5::digest::typenum::Integer;
use rust_decimal::Decimal;
use serde::de::IntoDeserializer;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use zip::ZipArchive; //read file into a string
                     //make function for split
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rm_daemon_utils::custom_serde;
use std::io::BufRead;
use std::io::Read;
use std::ops::Div;
// convert to struct
fn main() {
    let arg = env::args().nth(1).unwrap();
    match arg.as_str() {
        "by_network" | "bn" => {
            // some stuff here
            let file_271 = get_file_from_zip().unwrap();
            calculate_totals_bynetwork(&file_271)
        }
        "network_in_time" | "nw_in_t" => {
            let file_271 = get_file_from_zip().unwrap();
            calculate_totals_by_network_time(&file_271)
        }
        "average_demand" | "demand_avg" => {
            let file_0371 = combined_list_of_files(get_price_file());
            calculate_average(file_0371)
        }
        "help" | "-h" | "--help" => {
            println!("List of recent reports: \nby_network \naverage_demand \nnetwork_in_time")
        }
        "sensitivities" => {
            let sensitivities = download_sensitivities();
            let sens = convert_sensitivities(sensitivities);
            let avg = calculate_predispatch_average(sens);
            dbg!(avg);
        }
        "dispatch"|"disp" => {
            let dispatch = download_dispatch();
            let disp = convert_dispatch(dispatch);
            let avg = calculate_dispatch_average(disp);
            dbg!(avg);
            
        }
        other => {
            println!("Dont know, {other}")
        }
    }
}

// 1. Build essential structs

//1.1 Struct for 271
#[derive(Debug)]
struct Row {
    network_id: u16,
    _nsl_update: NaiveDateTime,
    _gas_date: NaiveDate,
    _ti: u16,
    nsl_gj: Decimal,
    _current_date: NaiveDateTime,
    network_month_and_year: NetworkAndMonthYear,
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
struct NetworkAndMonthYear(u16, u32, i32);

//1.2 Struct for 037b1

#[derive(Debug, Clone)]
struct Int037b {
    demand_type: String,
    price: Decimal,
    transmission_group_id: String,
    transmission_id: String,
    schedule_type: String,
    approval_date: NaiveDateTime,
    gas_date: NaiveDate,
    current_date: NaiveDateTime,
}

//function to get and open zip file
fn download_and_open_zip(url: &str) -> ZipArchive<io::Cursor<Vec<u8>>> {
    let response = reqwest::blocking::get(url)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    let cursor = std::io::Cursor::new(response);

    //step two: get the file out of zip
    let archive = zip::ZipArchive::new(cursor).unwrap();
    archive
}

//2. Work with int 271
//2.1. Function to get int271 file

fn get_file_from_zip() -> Option<String> {
    //Step one: download the file
    let url = "https://nemweb.com.au/Reports/Current/VicGas/CURRENTDAY.ZIP";

    let mut archive = download_and_open_zip(url).clone();

    for idx in 0..archive.len() {
        //iterate thru out all files
        let file = archive.by_index(idx).unwrap(); //get the file at position idx
        let filename = file.enclosed_name().unwrap().to_string_lossy().to_string(); //convert the file name into string
        if filename.contains("int271") {
            // if file name contains int271 then print out the file
            let bytes = file.bytes().map(|b| b.unwrap()).collect::<Vec<_>>(); //extract the bytes get the data
            return Some(String::from_utf8_lossy(&bytes).to_string()); // return the data in String
        }
    }

    None //if theres no int271 file name then return none

    // file_string
}

//2.2 Function for repeating code lines, to get data from the field
fn get_data_by_column(line: &str, idx: usize) -> &str {
    line.split(',').nth(idx).unwrap()
}
//2.3. Function to get data and put into struct
fn from_line(line: String) -> Row {
    // dbg!(&line);
    let network_id = get_data_by_column(&line, 0).parse().unwrap();
    let gas_date = NaiveDate::parse_from_str(get_data_by_column(&line, 2), "%d %b %Y").unwrap();
    Row {
        network_id,
        _nsl_update: NaiveDateTime::parse_from_str(
            get_data_by_column(&line, 1),
            "%d %b %Y %H:%M:%S",
        )
        .unwrap(),
        _gas_date: gas_date,
        _ti: get_data_by_column(&line, 3).parse().unwrap(),
        nsl_gj: get_data_by_column(&line, 4).parse().unwrap(),
        _current_date: NaiveDateTime::parse_from_str(
            get_data_by_column(&line, 5),
            "%d %b %Y %H:%M:%S",
        )
        .unwrap(),
        network_month_and_year: NetworkAndMonthYear(network_id, gas_date.month(), gas_date.year()),
    }
}

//2.4. Function to calculate totals by network

fn calculate_totals_bynetwork(file: &str) {
    let a = file.lines().skip(1);
    let mut totals3 = BTreeMap::new(); //dictionary of sum for each network id
    for row in a {
        let row = from_line(row.to_string());
        let current_value = totals3.get_mut(&row.network_id);
        match current_value {
            Some(mut i) => i += row.nsl_gj,

            None => {
                totals3.insert(row.network_id, row.nsl_gj);
            }
        };
    }
    println!("| network_id | totals     |");
    for (network_id, totals) in totals3 {
        let totals = totals.round();
        println!("|{network_id}           |  {totals:>10}|");
    }
}

//2.5. Function to calculate totals by month and year
fn calculate_totals_by_network_time(file: &str) {
    let a = file.lines().skip(1);
    let mut totals8 = BTreeMap::new(); //dictionary of sum for each network id
    for row in a {
        let row = from_line(row.to_string());
        let current_value = totals8.get_mut(&row.network_month_and_year);
        match current_value {
            Some(mut i) => i += row.nsl_gj,

            None => {
                totals8.insert(row.network_month_and_year, row.nsl_gj);
            }
        };
    }
    println!("|id| month  | year  | totals");
    for (network_and_month_year, totals) in totals8 {
        let totals = totals.round();
        println!(
            "|{0: <10}|{1: >10}|{2: >10}|{totals:>10}|",
            network_and_month_year.0, network_and_month_year.1, network_and_month_year.2
        );
    }
}

//3. Work with 037b
//3.1. function to get price file from zip
fn get_price_file() -> Vec<(String, String)> {
    //the result is a vector of tuple with file name and the file content [(file name,content)]
    //Step one: download the file
    let url2 = "https://nemweb.com.au/Reports/Current/VicGas/CURRENTDAY.ZIP";

    let mut archive = download_and_open_zip(url2).clone();

    let mut vec = Vec::new();
    for idx in 0..archive.len() {
        //iterate thru out all files
        let file = archive.by_index(idx).unwrap(); //get the file at position idx
        let filename = file.enclosed_name().unwrap().to_string_lossy().to_string(); //list of file name
        if filename.contains("int037b") {
            // if file name contains int271 then print out the file
            // println!(" File {}", filename);
            vec.push((
                filename,
                String::from_utf8_lossy(&file.bytes().map(|b| b.unwrap()).collect::<Vec<_>>())
                    .to_string(),
            ))
        }
    }

    vec //if theres no int271 file name then it returns none. This vector is a tuple of (filename and content)
        // [(file1_name,file1_content), (file_name2, file2_content),etc]
        // file_string
}

//This function convert the vector above into big vector - expected result [[(file, struct string)],[(file2, struct string)],[(fil3,struct string)]]
fn combined_list_of_files(files: Vec<(String, String)>) -> Vec<Vec<(String, Int037b)>> {
    // Create a vector that will contain vectors
    let mut list = Vec::new();

    for each_file in files {
        let mut rows = Vec::new();
        for line in each_file.1.lines().skip(1) {
            let row = from_line_037(line.to_string());
            rows.push((each_file.clone().0, row));
        }
        list.push(rows);
    }

    list
}

// this function take the values from data in the struct

fn from_line_037(line: String) -> Int037b {
    Int037b {
        demand_type: line.split(",").nth(0).unwrap().parse().unwrap(),
        price: line.split(",").nth(1).unwrap().parse().unwrap(),
        transmission_group_id: line.split(",").nth(2).unwrap().parse().unwrap(),
        transmission_id: line.split(",").nth(4).unwrap().parse().unwrap(),
        schedule_type: line.split(",").nth(3).unwrap().parse().unwrap(),
        gas_date: NaiveDate::parse_from_str(line.split(",").nth(5).unwrap(), "%d %b %Y").unwrap(),
        approval_date: NaiveDateTime::parse_from_str(
            line.split(",").nth(6).unwrap(),
            "%d %b %Y %H:%M:%S",
        )
        .unwrap(),
        current_date: NaiveDateTime::parse_from_str(
            line.split(",").nth(7).unwrap(),
            "%d %b %Y %H:%M:%S",
        )
        .unwrap(),
    }
}

//3.3.Function calculate average of price by demand_type
fn calculate_average(files: Vec<Vec<(String, Int037b)>>) {
    //start with the list files as argument, the big vector is just for learning purpose
    //create a vector that contain series of values of demand_type []
    let mut demand_types = files
        .iter() //flatten the vector, get into the nested vector
        .flatten() // f is (filename, content) so f.0 is file name f.1 is content
        .map(|f| f.clone().1.demand_type) //iter through the tuple, taking the field demand_type in file content, each content is a struct with demand_type as field
        .collect::<Vec<_>>(); //result [100%:list of values,90%: list of values, normal: list of values]
    demand_types.sort(); //sort the value
    demand_types.dedup(); //remove duplicates
                          //as we have a bunch of files, now we will apply the loop for each file
    for each_file in files.into_iter() {
        // files is the vector not yet the iterator
        println!(
            "|{0: <10}|{1: >10}|{2: <10}|",
            "demand_type", "average", "file name"
        ); //print headlines
        for demand_type in demand_types.clone() {
            //get file name
            let filename = each_file
                .clone()
                .iter()
                .map(|f| f.clone().0)
                .next()
                .unwrap();
            //get values of each file by demand type
            //make rows into vector so we can apply the method
            let rows = each_file
                .clone()
                .into_iter()
                .filter(|r| r.1.demand_type == demand_type)
                .map(|r| r.1.price)
                .collect::<Vec<_>>(); //copy each_file into the inner for loop bc each_file in outer for loop
            let len = Decimal::from(rows.len());
            let sum: Decimal = rows.iter().sum();
            let avg = sum.div(len).round_dp(4);
            println!("|{0: <10}|{1: >10}|{2: <10}|", demand_type, avg, filename);
        }
    }
}

// download this https://www.nemweb.com.au/REPORTS/CURRENT/Predispatch_Sensitivities as .text()

// find the last string that matches "PUBLIC_PREDISPATCH_SENSITIVITIES" (let file_name = ...;)
// use that to download the following
// https://www.nemweb.com.au/REPORTS/CURRENT/Predispatch_Sensitivities/{file_name}.zip

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Sensitivity {
    regionid: String,
    periodid: String,
    rrpeep1: Decimal,
    rrpeep2: Decimal,
    rrpeep3: Decimal,
    rrpeep4: Decimal,
    rrpeep5: Decimal,
    rrpeep6: Decimal,
    rrpeep7: Decimal,
    rrpeep8: Decimal,
    rrpeep9: Decimal,
    rrpeep10: Decimal,
    rrpeep11: Decimal,
    rrpeep12: Decimal,
    rrpeep13: Decimal,
    rrpeep14: Decimal,
    rrpeep15: Decimal,
    rrpeep16: Decimal,
    rrpeep17: Decimal,
    rrpeep18: Decimal,
    rrpeep19: Decimal,
    rrpeep20: Decimal,
    rrpeep21: Decimal,
    rrpeep22: Decimal,
    rrpeep23: Decimal,
    rrpeep24: Decimal,
    rrpeep25: Decimal,
    rrpeep26: Decimal,
    rrpeep27: Decimal,
    rrpeep28: Decimal,
    #[serde(deserialize_with = "custom_serde::predispatchdatetime_to_datetime::deserialize")]
    datetime: NaiveDateTime,
    #[serde(deserialize_with = "custom_serde::predispatchdatetime_to_datetime::deserialize")]
    lastchanged: NaiveDateTime,
    rrpeep29: Decimal,
    rrpeep30: Decimal,
    rrpeep31: Decimal,
    rrpeep32: Decimal,
    rrpeep33: Decimal,
    rrpeep34: Decimal,
    rrpeep35: Decimal,
    intervention_active: Decimal,
    rrpeep36: Decimal,
    rrpeep37: Decimal,
    rrpeep38: Decimal,
    rrpeep39: Decimal,
    rrpeep40: Decimal,
    rrpeep41: Decimal,
    rrpeep42: Decimal,
    rrpeep43: Decimal,
}

fn download_sensitivities() -> String {
    let response = reqwest::blocking::get(
        "https://www.nemweb.com.au/REPORTS/CURRENT/Predispatch_Sensitivities",
    )
    .unwrap()
    .text()
    .unwrap();
    let lines = response
        .split("<br>")
        .into_iter()
        .filter(|l| l.contains("PUBLIC_PREDISPATCH_SENSITIVITIES"))
        .last()
        .unwrap();
    let zip = lines[174..242].to_string();

    let url_sensitive = format!(
        "https://www.nemweb.com.au/REPORTS/CURRENT/Predispatch_Sensitivities/{}",
        zip
    );
    let file = download_and_open_zip(&url_sensitive)
        .by_index(0)
        .unwrap()
        .bytes()
        .map(|b| b.unwrap())
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&file).to_string()
}

fn convert_sensitivities(file: String) -> Vec<Sensitivity> {
    let lines = file.lines();
    let lines = lines
        .skip(1)
        .map(|l| l.to_string())
        .collect::<Vec<String>>();
    let lines = lines.join("\n");
    let mut reader = csv::Reader::from_reader(lines.as_bytes());
    let mut vec = Vec::new();
    for row in reader.deserialize() {
        vec.push(row);
    }
    vec.into_iter().filter_map(|r| r.ok()).collect()
}

//Calculate average

fn calculate_predispatch_average(vec: Vec<Sensitivity>) {
    let mut regions = vec
        .clone()
        .into_iter()
        .map(|v| v.regionid)
        .collect::<Vec<_>>();
    regions.sort();
    regions.dedup();
    let mut dates = vec
        .clone()
        .into_iter()
        .map(|d| d.datetime)
        .collect::<Vec<_>>();
    dates.sort();
    dates.dedup();

    println!(
        "|{0: ^10}|{1: ^20}|{2: ^24}|{3: ^24}|",
        "region", "-100", "-500", "date"
    );
    for date in dates.clone().into_iter() {
        for region in regions.clone() {
            if region.contains("NSW") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| r.regionid == region && r.datetime == date)
                    .map(|r| (r.rrpeep1, r.rrpeep5))
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum1: Decimal = row.iter().map(|r| r.0).sum();
                let avg1 = sum1.div(len).round_dp(4);

                let sum2: Decimal = row.iter().map(|r| r.1).sum();
                let avg2 = sum2.div(len).round_dp(4);

                let date = date.to_string();
                println!(
                    "|{0: ^10}|{1: ^20}|{2: ^24}|{3: ^24}|",
                    region, avg1, avg2, date
                );
            } else if region.contains("VIC") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| r.regionid == region && r.datetime == date)
                    .map(|r| (r.rrpeep8, r.rrpeep12))
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum1: Decimal = row.iter().map(|r| r.0).sum();
                let avg1 = sum1.div(len).round_dp(4);

                let sum2: Decimal = row.iter().map(|r| r.1).sum();
                let avg2 = sum2.div(len).round_dp(4);

                let date = date.to_string();
                println!(
                    "|{0: ^10}|{1: ^20}|{2: ^24}|{3: ^24}|",
                    region, avg1, avg2, date
                );
            } else if region.contains("QLD") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| r.regionid == region && r.datetime == date)
                    .map(|r| (r.rrpeep29, r.rrpeep33))
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum1: Decimal = row.iter().map(|r| r.0).sum();
                let avg1 = sum1.div(len).round_dp(4);

                let sum2: Decimal = row.iter().map(|r| r.1).sum();
                let avg2 = sum2.div(len).round_dp(4);
                let date = date.to_string();
                println!(
                    "|{0: ^10}|{1: ^20}|{2: ^24}|{3: ^24}|",
                    region, avg1, avg2, date
                );
            } else if region.contains("SA") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| r.regionid == region && r.datetime == date)
                    .map(|r| (r.rrpeep17, r.rrpeep19))
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum1: Decimal = row.iter().map(|r| r.0).sum();
                let avg1 = sum1.div(len).round_dp(4);

                let sum2: Decimal = row.iter().map(|r| r.1).sum();
                let avg2 = sum2.div(len).round_dp(4);
                let date = date.to_string();
                println!(
                    "|{0: ^10}|{1: ^20}|{2: ^24}|{3: ^24}|",
                    region, avg1, avg2, date
                );
            }
        }
    }
}

// WORK WITH DISPATCH FILES (PUBLIC PRICES)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Dispatch {
    #[serde(deserialize_with = "custom_serde::predispatchdatetime_to_datetime::deserialize")]
    settlementdate: NaiveDateTime,
    regionid: String,
    intervention: String,
    rrp: Decimal,
    eep: Decimal,
    rop: Decimal,
    // apcflag: Decimal,
    // marketsuspendedflag: Decimal,
    totaldemand: Decimal,
    demandforecast: Decimal,
    dispatchablegeneration: Decimal,
    dispatchableload: Decimal,
    netinterchange: Option<Decimal>,
    // excessgeneration: Option<Decimal>,
    // lower5_mindispatch: Option<Decimal>,
    // lower5_minimport: Option<Decimal>,
    lower5_minlocaldispatch: Option<Decimal>,
    // lower5_minlocalprice: Option<Decimal>,
    // lower5_minlocalreq: Option<Decimal>,
    // lower5_minprice: Option<Decimal>,
    // lower5_minreq: Option<Decimal>,
    // lower5_minsupplyprice: Option<Decimal>,
    // lower60_secdispatch: Option<Decimal>,
    // lower60_secimport: Option<Decimal>,
    lower60_seclocaldispatch: Option<Decimal>,
    // lower60_seclocalprice: Option<Decimal>,
    // lower60_seclocalreq: Option<Decimal>,
    // lower60_secprice: Option<Decimal>,
    // lower60_secreq: Option<Decimal>,
    // lower60_secsupplyprice: Option<Decimal>,
    // lower6_secdispatch: Option<Decimal>,
    // lower6_secimport: Option<Decimal>,
    lower6_seclocaldispatch: Option<Decimal>,
    // lower6_seclocalprice: Option<Decimal>,
    // lower6_seclocalreq: Option<Decimal>,
    // lower6_secprice: Option<Decimal>,
    // lower6_secreq: Option<Decimal>,
    // lower6_secsupplyprice: Option<Decimal>,
    // raise5_mindispatch: Option<Decimal>,
    // raise5_minimport: Option<Decimal>,
    raise5_minlocaldispatch: Option<Decimal>,
    // raise5_minlocalprice: Option<Decimal>,
    // raise5_minlocalreq: Option<Decimal>,
    // raise5_minprice: Option<Decimal>,
    // raise5_minreq: Option<Decimal>,
    // raise5_minsupplyprice: Option<Decimal>,
    // raise60_secdispatch: Option<Decimal>,
    // raise60_secimport: Option<Decimal>,
    raise60_seclocaldispatch: Option<Decimal>,
    // raise60_seclocalprice: Option<Decimal>,
    // raise60_seclocalreq: Option<Decimal>,
    // raise60_secprice: Option<Decimal>,
    // raise60_secreq: Option<Decimal>,
    // raise60_secsupplyprice: Option<Decimal>,
    // raise6_secdispatch: Option<Decimal>,
    // raise6_secimport: Option<Decimal>,
    raise6_seclocaldispatch: Option<Decimal>,
    // raise6_seclocalprice: Option<Decimal>,
    // raise6_seclocalreq: Option<Decimal>,
    // raise6_secprice: Option<Decimal>,
    // raise6_secreq: Option<Decimal>,
    // raise6_secsupplyprice: Option<Decimal>,
    aggregatedispatcherror: Option<Decimal>,
    availablegeneration: Option<Decimal>,
    availableload: Option<Decimal>,
    initialsupply: Option<Decimal>,
    clearedsupply: Option<Decimal>,
    // lowerregimport: Option<Decimal>,
    lowerreglocaldispatch: Option<Decimal>,
    // lowerreglocalreq: Option<Decimal>,
    // lowerregreq: Option<Decimal>,
    // raiseregimport: Option<Decimal>,
    raisereglocaldispatch: Option<Decimal>,
    // raisereglocalreq: Option<Decimal>,
    // raiseregreq: Option<Decimal>,
    // raise5_minlocalviolation: Option<Decimal>,
    // raisereglocalviolation: Option<Decimal>,
    // raise60_seclocalviolation: Option<Decimal>,
    // raise6_seclocalviolation: Option<Decimal>,
    // lower5_minlocalviolation: Option<Decimal>,
    // lowerreglocalviolation: Option<Decimal>,
    // lower60_seclocalviolation: Option<Decimal>,
    // lower6_seclocalviolation: Option<Decimal>,
    raise5_minviolation: Option<Option<Decimal>>,
    raiseregviolation: Option<Option<Decimal>>,
    raise60_secviolation: Option<Decimal>,
    raise6_secviolation: Option<Decimal>,
    lower5_minviolation: Option<Decimal>,
    lowerregviolation: Option<Decimal>,
    lower60_secviolation: Option<Decimal>,
    lower6_secviolation: Option<Decimal>,
    raise6_secrrp: Option<Decimal>,
    raise6_secrop: Option<Decimal>,
    raise6_secapcflag: Option<Decimal>,
    raise60_secrrp: Option<Decimal>,
    raise60_secrop: Option<Decimal>,
    raise60_secapcflag: Option<Decimal>,
    raise5_minrrp: Option<Decimal>,
    raise5_minrop: Option<Decimal>,
    raise5_minapcflag: Option<Decimal>,
    raiseregrrp: Option<Decimal>,
    raiseregrop: Option<Decimal>,
    raiseregapcflag: Option<Decimal>,
    lower6_secrrp: Option<Decimal>,
    lower6_secrop: Option<Decimal>,
    lower6_secapcflag: Option<Decimal>,
    lower60_secrrp: Option<Decimal>,
    lower60_secrop: Option<Decimal>,
    lower60_secapcflag: Option<Decimal>,
    lower5_minrrp: Option<Decimal>,
    lower5_minrop: Option<Decimal>,
    lower5_minapcflag: Option<Decimal>,
    lowerregrrp: Option<Decimal>,
    lowerregrop: Option<Decimal>,
    lowerregapcflag: Option<Decimal>,
}

fn download_dispatch() -> String {
    let response =
        reqwest::blocking::get("https://www.nemweb.com.au/REPORTS/CURRENT/Public_Prices/")
            .unwrap()
            .text()
            .unwrap();
    let line = response
        .split("<br>")
        .into_iter()
        .filter(|l| l.contains("PUBLIC_PRICES"))
        .last()
        .unwrap();
    let zipname = line[139..184].to_string();
    let url_dispatch = format! {
        "https://www.nemweb.com.au/REPORTS/CURRENT/Public_Prices/{}",zipname
    };
    //use the function to download and open the zip
    let file = download_and_open_zip(&url_dispatch)
        .by_index(0)
        .unwrap()
        .bytes()
        .map(|b| b.unwrap())
        .collect::<Vec<_>>(); //because we only have 1 file so take the by_index(0)
    String::from_utf8_lossy(&file).to_string() // convert the file from vec<u8> to string
}

fn convert_dispatch(file: String) -> Vec<Dispatch> {
    let lines = file.lines();
    let lines = lines
        .skip(1)
        .map(|l| l.to_string())
        .collect::<Vec<String>>();
    let lines = lines.join("\n");
    let mut reader = csv::Reader::from_reader(lines.as_bytes());

    let mut vec = Vec::new();
    for row in reader.deserialize() {
        match row {
            Ok(r) => vec.push(r),
            Err(e) => {}
        }
    }
    // vec.into_iter().filter_map(|r| r.ok()).collect()
    vec
}

fn calculate_dispatch_average(vec: Vec<Dispatch>) {
    //create a vector with unique regions
    let mut regions = vec
        .clone()
        .into_iter()
        .map(|v| v.regionid)
        .collect::<Vec<_>>();
    //sort and remove the duplicate
    regions.sort();
    regions.dedup();
    //create a vector with unique dates
    let mut dates = vec
        .clone()
        .into_iter()
        .map(|v| v.settlementdate)
        .collect::<Vec<_>>();
    //sort and remove the duplicate
    dates.sort();
    dates.dedup();

    // print headline
    println!("|{0: ^10}|{1: ^20}|{2: ^24}|", "region", "dispatch", "date");

    for date in dates.iter() {
        for region in regions.iter() {
            if region.contains("NSW") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| &r.regionid == region&& &r.settlementdate == date)
                    .map(|r| r.rrp)
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len()); //convert the length into decimal
                let sum: Decimal = row.iter().sum();
                let avg = sum.div(len).round_dp(4);
                println!("|{0: ^10}|{1: ^20}|{2: ^24}|", region, avg, date);
            }
            if region.contains("VIC") {
                let row = vec
                    .clone()
                    .into_iter()
                    .filter(|r| &r.regionid == region&& &r.settlementdate == date)
                    .map(|r| r.rrp)
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len()); //convert the length into decimal
                let sum: Decimal = row.iter().sum();
                let avg = sum.div(len).round_dp(4);
                println!("|{0: ^10}|{1: ^20}|{2: ^24}|", region, avg, date);
            }
            if region.contains("QLD") {
                let row = vec
                    .iter()
                    .filter(|r| &r.regionid == region&& &r.settlementdate == date)
                    .map(|r| r.rrp)
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum: Decimal = row.iter().sum();
                let avg = sum.div(len).round_dp(4);
                println!("|{0: ^10}|{1: ^20}|{2: ^24}|", region, avg, date);
            }
            if region.contains("SA") {
                let row = vec
                    .iter()
                    .filter(|r| &r.regionid == region&& &r.settlementdate == date)
                    .map(|r| r.rrp)
                    .collect::<Vec<_>>();
                let len = Decimal::from(row.len());
                let sum: Decimal = row.iter().sum();
                let avg = sum.div(len).round_dp(4);
                println!("|{0: ^10}|{1: ^20}|{2: ^24}|", region, avg, date);
            }
        }
    }
}
