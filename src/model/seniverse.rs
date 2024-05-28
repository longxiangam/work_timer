use alloc::string::ToString;
use esp_println::println;
use heapless::{String, Vec};
use lite_json::parse_json;
//三日
#[derive( Debug)]
pub struct  DailyResponse{
    pub results:Vec<DailyResult,1>
}
#[derive( Debug,Default)]
pub struct  DailyResult{
    pub last_update:String<40>,
    pub daily:Vec<Daily,5>,
    pub location: Location
}
#[derive( Debug,Default)]
pub struct Location{
    pub id: String<20>,
    pub name: String<20>,
    pub country: String<20>,
    pub path:  String<50>,
    pub timezone: String<30>,
    pub timezone_offset: String<20>
}
#[derive( Debug,Default)]
pub struct Daily{
    pub  date:String<20>,
    pub  text_day: String<20>,
    pub code_day: String<20>,
    pub text_night:String<20>,
    pub  code_night:String<20>,
    pub high: String<20>,
    pub low: String<20>,
    pub rainfall:String<20>,
    pub precip: String<20>,
    pub wind_direction: String<20>,
    pub wind_direction_degree: String<20>,
    pub wind_speed: String<20>,
    pub wind_scale: String<20>,
    pub humidity: String<20>,
}
macro_rules! set_field {
    ($temp:ident, $one:ident,$key:ident, $($key_liter:literal => $field:ident),*) => {
        $(
            if $key.eq($key_liter) {
                $temp.$field = $one.1.as_string().unwrap().iter().collect();
            }
        )*
    };
}

//lite_json::parse_json 中有中文会报错
pub fn form_json(str:&[u8])->Option<DailyResponse>{
    let result = parse_json(core::str::from_utf8(str).unwrap());
    match result {
        Ok(json_data) => {
            println!("json:{:?}", json_data.is_object());
            let mut location:Option<Location> = None;
            let mut daily:Vec<Daily,5 > = Vec::new();
            let mut lastUpdate:String<40> = String::new();
            if json_data.is_object() {
                //results
                for result_ in json_data.as_object().unwrap().iter() {
                    //results array
                    if result_.1.is_array() {
                        for result__ in result_.1.as_array().unwrap().iter() {
                            println!("key:{:?},value:{:?}", result_.0, result_.1.is_array());

                            //遍历对象的所有属性
                            for one in result__.as_object().unwrap().iter() {
                                println!("key:{:?},value:{:?}", one.0, one.1.is_array());
                                let key: String<40> = one.0.iter().collect();
                                if key.eq("location") {
                                    let mut temp = Location::default();

                                    for one in one.1.as_object().unwrap().iter() {
                                        let key: String<40> = one.0.iter().collect();
                                        if key.eq("id") {
                                            temp.id = one.1.as_string().unwrap().iter().collect();
                                        }
                                        if key.eq("name") {
                                            temp.name = one.1.as_string().unwrap().iter().collect();
                                        }
                                        if key.eq("country") {
                                            temp.country = one.1.as_string().unwrap().iter().collect();
                                        }
                                        if key.eq("path") {
                                            temp.path = one.1.as_string().unwrap().iter().collect();
                                        }
                                        if key.eq("timezone") {
                                            temp.timezone = one.1.as_string().unwrap().iter().collect();
                                        }
                                        if key.eq("timezone_offset") {
                                            temp.timezone_offset = one.1.as_string().unwrap().iter().collect();
                                        }
                                    }
                                    location = Some(temp);
                                }

                                if key.eq("daily") {
                                    for one_daily in one.1.as_array().unwrap() {
                                        let mut temp = Daily::default();
                                        for one_daily_ in one_daily.as_object().unwrap().iter() {
                                            let key: String<40> = one_daily_.0.iter().collect();
                                            set_field!(temp,one_daily_,key
                                            ,"date"=>date
                                            ,"text_day"=>text_day
                                            ,"code_day"=>code_day
                                            ,"text_night"=>text_night
                                            ,"code_night"=>code_night
                                            ,"high"=>high
                                            ,"low"=>low
                                            ,"rainfall"=>rainfall
                                            ,"precip"=>precip
                                            ,"wind_direction"=>wind_direction
                                            ,"wind_direction_degree"=>wind_direction_degree
                                            ,"wind_speed"=>wind_speed
                                            ,"wind_scale"=>wind_scale
                                            ,"humidity"=>humidity
                                        );
                                        }

                                        daily.push(temp);
                                    }
                                }

                                if key.eq("last_update") {
                                    lastUpdate = one.1.as_string().unwrap().iter().collect();
                                }
                            }
                        }
                    }
                }
            }

            let mut results:DailyResult = Default::default();
            if let Some(v) = location {
                results.location = v;
                results.daily = daily;
                results.last_update = lastUpdate;

                let mut vec:Vec<DailyResult,1> = Vec::new();
                vec.push(results);
                let  daily_response = DailyResponse{ results:vec};

                println!("format:{:?}",daily_response);
                Some(daily_response)
            }else {
                None
            }
        }
        Err(e) => {
            println!("json error:{:?}",e);
            None
        }
    }
}


//实时

