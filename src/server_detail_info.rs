use crate::util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::{self, number::complete::*, *};
use std;
use std::collections::HashMap;
use std::ffi::CString;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NetworkVehicleType {
    Train,
    Lorry,
    Bus,
    Plane,
    Ship,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompanyInfo {
    pub index: u8,
    pub name: CString,
    pub inaugurated_year: u32,
    pub company_value: u64,
    pub money: u64,
    pub income: u64,
    pub performance_history: u16,
    pub has_password: bool,
    pub num_vehicles: HashMap<NetworkVehicleType, u16>,
    pub num_stations: HashMap<NetworkVehicleType, u16>,
    pub is_ai: bool,
}

impl CompanyInfo {
    pub fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u8(self.index)?;
        buf.append(&mut self.name.clone().into_bytes_with_nul());
        buf.write_u32::<LittleEndian>(self.inaugurated_year)?;
        buf.write_u64::<LittleEndian>(self.company_value)?;
        buf.write_u64::<LittleEndian>(self.money)?;
        buf.write_u64::<LittleEndian>(self.income)?;
        buf.write_u16::<LittleEndian>(self.performance_history)?;
        buf.write_u8(if self.has_password { 1 } else { 0 })?;

        buf.write_u16::<LittleEndian>(
            *self
                .num_vehicles
                .get(&NetworkVehicleType::Train)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_vehicles
                .get(&NetworkVehicleType::Lorry)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_vehicles
                .get(&NetworkVehicleType::Bus)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_vehicles
                .get(&NetworkVehicleType::Plane)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_vehicles
                .get(&NetworkVehicleType::Ship)
                .unwrap_or(&0),
        )?;

        buf.write_u16::<LittleEndian>(
            *self
                .num_stations
                .get(&NetworkVehicleType::Train)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_stations
                .get(&NetworkVehicleType::Lorry)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_stations
                .get(&NetworkVehicleType::Bus)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_stations
                .get(&NetworkVehicleType::Plane)
                .unwrap_or(&0),
        )?;
        buf.write_u16::<LittleEndian>(
            *self
                .num_stations
                .get(&NetworkVehicleType::Ship)
                .unwrap_or(&0),
        )?;

        buf.write_u8(if self.is_ai { 1 } else { 0 })?;

        Ok(())
    }
}

named!(pub parse_company_info<&[u8], CompanyInfo>,
    do_parse!(
        index: le_u8 >>
        name: read_cstring >>
        inaugurated_year: le_u32 >>
        company_value: le_u64 >>
        money: le_u64 >>
        income: le_u64 >>
        performance_history: le_u16 >>
        has_password: map!(le_u8, |v| v > 0) >>

        num_vehicles_train: le_u16 >>
        num_vehicles_lorry: le_u16 >>
        num_vehicles_bus: le_u16 >>
        num_vehicles_plane: le_u16 >>
        num_vehicles_ship: le_u16 >>

        num_stations_train: le_u16 >>
        num_stations_lorry: le_u16 >>
        num_stations_bus: le_u16 >>
        num_stations_plane: le_u16 >>
        num_stations_ship: le_u16 >>

        is_ai: map!(le_u8, |v| v > 0) >>
        (CompanyInfo {
            index,
            name,
            inaugurated_year,
            company_value,
            money,
            income,
            performance_history,
            has_password,
            num_vehicles: hashmap! {
                NetworkVehicleType::Train => num_vehicles_train,
                NetworkVehicleType::Lorry => num_vehicles_lorry,
                NetworkVehicleType::Bus => num_vehicles_bus,
                NetworkVehicleType::Plane => num_vehicles_plane,
                NetworkVehicleType::Ship => num_vehicles_ship,
            },
            num_stations: hashmap! {
                NetworkVehicleType::Train => num_stations_train,
                NetworkVehicleType::Lorry => num_stations_lorry,
                NetworkVehicleType::Bus => num_stations_bus,
                NetworkVehicleType::Plane => num_stations_plane,
                NetworkVehicleType::Ship => num_stations_ship,
            },
            is_ai,
        })
    )
);

#[derive(Clone, Debug, PartialEq)]
pub struct ServerDetailInfo {
    pub company_info_version: u8,
    pub companies: Vec<CompanyInfo>,
}

impl ServerDetailInfo {
    pub fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u8(self.company_info_version)?;
        buf.write_u8(self.companies.len() as u8)?;
        for company in self.companies.iter() {
            company.write_pkt(buf)?;
        }

        Ok(())
    }
}

named!(pub parse_server_detail_info<&[u8], ServerDetailInfo>,
    do_parse!(
        company_info_version: le_u8 >>
        company_count: le_u8 >>
        companies: count!(parse_company_info, company_count as usize) >>
        (ServerDetailInfo {
            company_info_version, companies
        })
    )
);
