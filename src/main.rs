extern crate reqwest;
extern crate minidom;
extern crate serde;
extern crate serde_json;
extern crate regex;

use reqwest::Error;
// use reqwest::header::{Authorization, Basic, Error};
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use regex::Regex;

#[derive(Default, PartialEq, Debug)]
struct SharePayload {
    shares_15m: f64,
    shares_1d: f64
}

#[derive(Serialize, Deserialize)]
struct HuobiPoolResp {
    code: u32,
	data: HuobiPoolData,
	message: String,
	success: bool
}

#[derive(Serialize, Deserialize)]
struct HuobiPoolData {
    speed_f: String,
    speed_s: String,
    speed_t: String
}

#[derive(Serialize, Deserialize)]
struct BtcPoolResp {
    err_no: u32,
	data: BtcPoolData
}

#[derive(Serialize, Deserialize)]
struct BtcPoolData {
    shares_15m: Value,
    shares_15m_unit: String,
    shares_1d: Value,
    shares_1d_unit: String
}

fn monitor_btcpool(url: &str) -> Result<SharePayload, Error> {
	// https://pool.btc.com/dashboard?access_key=r_qQPTVtQcZDylU&puid=435429
	let body: String = reqwest::get(url)?.text()?;
	let obj: BtcPoolResp = serde_json::from_str(&body).unwrap();
	
	println!("=== btcpool ===");
	// println!("shares_15m:{}{}, shares_1d:{}{}",
	// 	obj.data.shares_15m, obj.data.shares_15m_unit,
	// 	obj.data.shares_1d, obj.data.shares_1d_unit);
	
	let mut shares_15m;
	if obj.data.shares_15m.is_string() {
			shares_15m = obj.data.shares_15m.as_str().unwrap().parse::<f64>().unwrap();
		}else{
			shares_15m = obj.data.shares_15m.as_f64().unwrap();
		}
	
	let mut shares_1d;
	if obj.data.shares_1d.is_string() {
			shares_1d = obj.data.shares_1d.as_str().unwrap().parse::<f64>().unwrap();
		}else{
			shares_1d = obj.data.shares_1d.as_f64().unwrap();
		}
	
	match obj.data.shares_15m_unit.as_ref() {
		"T" => {shares_15m /= 1024.0},
		_ => {},
	}
	
	match obj.data.shares_1d_unit.as_ref() {
		"T" => {shares_1d /= 1024.0},
		_ => {},
	}
	
	let payload = SharePayload {
        shares_15m: shares_15m,
        shares_1d: shares_1d
    };
	
    Ok(payload)
}

#[derive(Serialize, Deserialize)]
struct SpiderPoolResp {
    code: String,
	data: SpiderPoolWorker
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct SpiderPoolWorker {
	hashrate15Fmt: Option<SpiderPoolHashData>,
    hashrate1440Fmt: Option<SpiderPoolHashData>
}

#[derive(Serialize, Deserialize)]
struct SpiderPoolHashData {
    value: f64,
    unit: String
}

fn monitor_spiderpool(url: &str) -> Result<SharePayload, Error> {
	// https://www.spiderpool.com/coin/show/btc/bmytest1/detail.html
	let re = Regex::new(r"(?m)https://www.spiderpool.com/coin/show/btc/(?P<account>[a-zA-Z0-9]+?)/detail.html").unwrap();
	let caps = re.captures(url).unwrap();
	let account = &caps["account"].to_lowercase();

	let api_url = format!("https://btc.api.spiderpool.com:19101/v1/subaccount/status?coin=btc&subaccount={}", account);
	let client = reqwest::Client::new();
	let body: String = client.get(&api_url).send()?.text()?;
	// println!("{}", body);

	// let body: String = reqwest::get(api_url)?.text()?;
	let obj: SpiderPoolResp = serde_json::from_str(&body).expect("have no data.");
	println!("=== spiderpool ===");
	
	let mut shares_15m = 0.0;
	if obj.data.hashrate15Fmt.is_some(){
		let hashrate15Fmt = obj.data.hashrate15Fmt.unwrap();
		shares_15m = hashrate15Fmt.value;
		match hashrate15Fmt.unit.as_ref() {
			"TH" => {shares_15m /= 1024.0},
			_ => {},
		}
	}

	let mut shares_1d = 0.0;
	if obj.data.hashrate1440Fmt.is_some(){
		let hashrate1440Fmt = obj.data.hashrate1440Fmt.unwrap();
		shares_1d = hashrate1440Fmt.value;
		match hashrate1440Fmt.unit.as_ref() {
			"TH" => {shares_1d /= 1024.0},
			_ => {},
		}
	}

	let payload = SharePayload {
        shares_15m: shares_15m,
        shares_1d: shares_1d
    };
	
	Ok(payload)
}

#[derive(Serialize, Deserialize)]
struct PoolinResp {
    err_no: u32,
	data: PoolinHashData
}

#[derive(Serialize, Deserialize)]
struct PoolinHashData {
    workers_active: u32,
	workers_inactive: u32,
	workers_dead: u32,
	workers_total: u32,
	shares_15m: f64,
	shares_24h: f64,
	shares_unit: String,
}

fn monitor_poolin(url: &str) -> Result<SharePayload, Error> {
	//https://www.poolin.com/my/9052047/btc/miners?read_token=wowkUwojrU5GAwJXmEn30tqpRvmBwdPdAKR6paQn2AMZbZr2I3yrYa6usqvrLOHa&status=ACTIVE
	let re = Regex::new(r"(?m)https://[[:ascii:]]+?/my/(?P<puid>\d+?)/btc/miners\?read_token=(?P<reader_token>[a-zA-Z0-9_]+)&status=ACTIVE").unwrap();
	let caps = re.captures(url).unwrap();
	
	// println!("{:#?}", caps);
	// let reader_token = r"wowkUwojrU5GAwJXmEn30tqpRvmBwdPdAKR6paQn2AMZbZr2I3yrYa6usqvrLOHa";
	// println!("{} {}", puid, reader_token);
	let puid = &caps["puid"];
	let reader_token = &caps["reader_token"];
	
	// https://api-prod.poolin.com/api/public/v2/worker/stats?puid=9052047&coin_type=btc
	let api_url = format!("https://api-prod.poolin.com/api/public/v2/worker/stats?puid={}&coin_type=btc", puid);
	let client = reqwest::Client::new();
	let body: String = client.get(&api_url).bearer_auth(reader_token).send()?.text()?;
	let obj: PoolinResp = serde_json::from_str(&body).unwrap();

	println!("=== poolin ===");
	// println!("workers_active {}", obj.data.workers_active);
	// 	println!("shares_15m {}", obj.data.shares_15m);
	// 	println!("shares_24h {}", obj.data.shares_24h);

	let mut shares_15m = obj.data.shares_15m;
	match obj.data.shares_unit.as_ref() {
		"T" => {shares_15m /= 1024.0},
		_ => {},
	}

	let mut shares_1d = obj.data.shares_24h;
	match obj.data.shares_unit.as_ref() {
		"T" => {shares_1d /= 1024.0},
		_ => {},
	}

	let payload = SharePayload {
        shares_15m: shares_15m,
        shares_1d: shares_1d
    };

	Ok(payload)
}

fn monitor_huobipool(url: &str) -> Result<SharePayload, Error> {
	// https://www.huobipool.com/pow/miners?signature=n4lnoQ2-mra7NoJxoRbEdA
	// "data":{"speed_f":"0.0000000000","speed_s":"0.0000000000","speed_t":"0.0000000000"},
	let body: String = reqwest::get(url)?.text()?;
	let obj: HuobiPoolResp = serde_json::from_str(&body).unwrap();
		
	let payload = SharePayload {
        shares_15m: obj.data.speed_f.parse::<f64>().unwrap(),
        shares_1d: obj.data.speed_t.parse::<f64>().unwrap()
    };
	// println!("speed_f {}", format_hashrate(obj.data.speed_f.parse().unwrap()));
	// println!("speed_s {}", format_hashrate(obj.data.speed_s.parse().unwrap()));
	// println!("speed_t {}", format_hashrate(obj.data.speed_t.parse().unwrap()));
		
	Ok(payload)
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct AntPoolResp {
    userGroupList: String,
	userWorkerList: AntPoolUserWorker
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct AntPoolUserWorker {
	useWorkerId: String,
    hsLash1d: String,
	hsLash1h: String,
	hsLast5m: String,
	rejectRate: f64,
}

#[allow(dead_code)]
fn monitor_antpool(url: &str) -> Result<SharePayload, Error> {
	// https://www.antpool.com/observer.htm?accessKey=3iaNEfO5tm4y0iLLGaiO&coinType=BTC
	println!("=== antpool ===");
	
	let body: String = reqwest::get(url)?.text()?;
	let _obj: AntPoolResp = serde_json::from_str(&body).unwrap();
		
	let payload = SharePayload {
        shares_15m: 0.0,
        shares_1d: 0.0
    };
	// println!("speed_f {}", format_hashrate(obj.data.speed_f.parse().unwrap()));
	// println!("speed_s {}", format_hashrate(obj.data.speed_s.parse().unwrap()));
	// println!("speed_t {}", format_hashrate(obj.data.speed_t.parse().unwrap()));
		
	Ok(payload)
}

fn main() -> Result<(), Error> {
	let mut total_share = SharePayload::default();
				
	println!("=== wangp001 280T ===");
	// _ret = monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_R6O5Znbu62Rbc&puid=438908");
	match monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_R6O5Znbu62Rbc&puid=438908") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("=== qm001 1400T ===");
	// _ret = monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_yPX9uJAUr1O3y&puid=438907");
	match monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_yPX9uJAUr1O3y&puid=438907") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };

	println!("=== YY321 3570T ===");
	// _ret = monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_zeMvLldrHZBNG&puid=437981");
	match monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_zeMvLldrHZBNG&puid=437981") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("=== ZW321 350T ===");
	// https://pool.btc.com/dashboard?access_key=r_a0xiXY5TCNsY7&puid=443548
	match monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_a0xiXY5TCNsY7&puid=443548") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	// println!("=== Yan09 0T ===");
// 	// _ret = monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_Dg8RMLPjB7V7h&puid=438684");
// 	match monitor_btcpool("https://pool.btc.com/v1/realtime/hashrate?access_key=r_Dg8RMLPjB7V7h&puid=438684") {
//         Ok(payload) => {
//         	println!("payload: {:?}", payload);
// 			total_share.shares_15m += payload.shares_15m;
// 			total_share.shares_1d += payload.shares_1d;
//         },
// 		Err(err) => eprintln!("error: {}", err),
//     };
	
	println!("=== bmytest1 2030T ===");
	match monitor_spiderpool("https://www.spiderpool.com/coin/show/btc/bmytest1/detail.html") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("total_share: {:?}", total_share);
	
	// #########################################
				
	total_share = SharePayload::default();

	println!("=== gaopengpeng0809 1P ===");	
	match monitor_spiderpool("https://www.spiderpool.com/coin/show/btc/gaopengpeng0809/detail.html") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("=== Funny 3.5P ===");	
	match monitor_spiderpool("https://www.spiderpool.com/coin/show/btc/Funny/detail.html") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("=== yibobtc06 0.5P  ===");	
	match monitor_spiderpool("https://www.spiderpool.com/coin/show/btc/yibobtc06/detail.html") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	println!("total_share: {:?}", total_share);
	// #########################################
				
	total_share = SharePayload::default();

	println!("=== huobipool 10T ===");	
	match monitor_huobipool("https://www.huobipool.com/p4/pow/sub_user_speed?visitor_path=n4lnoQ2-mra7NoJxoRbEdA&currency=btc") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };

	println!("=== poolin 20T ===");	
	match monitor_poolin("https://www.poolin.com/my/9052047/btc/miners?read_token=wowkUwojrU5GAwJXmEn30tqpRvmBwdPdAKR6paQn2AMZbZr2I3yrYa6usqvrLOHa&status=ACTIVE") {
        Ok(payload) => {
        	println!("payload: {:?}", payload);
			total_share.shares_15m += payload.shares_15m;
			total_share.shares_1d += payload.shares_1d;
        },
		Err(err) => eprintln!("error: {}", err),
    };
	
	// println!("=== antpool 20T ===");
	// match monitor_antpool("https://www.antpool.com/observer.htm?m=minerManage&accessKey=3iaNEfO5tm4y0iLLGaiO") {
	//         Ok(payload) => {
	//         	println!("payload: {:?}", payload);
	// 		total_share.shares_15m += payload.shares_15m;
	// 		total_share.shares_1d += payload.shares_1d;
	//         },
	// 	Err(err) => eprintln!("error: {}", err),
	//     };
	//
	// println!("total_share: {:?}", total_share);
	
	Ok(())
}
