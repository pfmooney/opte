//! The Virtual Private Cloud (VPC) represents the overlay network
//! that guests operate on. It presents a virtual L3 (IPv4 and IPv6)
//! network on top of the Oxide Rack's physical IPv6 network.
//!
//! TODO Discuss how VPC subnets work within VPC and related to guest
//! interfaces.
//!
//! Relevant RFD sections
//!
//! * RFD 21 User Networking API
//! ** §2.2 VPC Subnets
//! * RFD 63 Network Architecture
//! ** §3.1 Mappings to User API Concepts
use crate::ip4::{IpError, Ipv4Addr, Ipv4Cidr};

#[cfg(all(not(feature = "std"), not(test)))]
use alloc::prelude::v1::*;

#[cfg(any(feature = "std", test))]
use std::prelude::v1::*;

use std::result;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// RFD 21 §2.2
pub const OXIDE_MIN_IP4_BLOCK: u8 = 26;

pub type Result<T> = result::Result<T, IpError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VpcSubnet4 {
    cidr: Ipv4Cidr,
}

impl VpcSubnet4 {
    pub fn from_req(req: SetVpcSubnet4Req) -> Result<Self> {
        req.vpc_sub_cidr.parse()
    }

    pub fn get_cidr(&self) -> Ipv4Cidr {
        self.cidr
    }

    /// Is this `ip` a member of the subnet?
    pub fn is_member(&self, ip: Ipv4Addr) -> bool {
        self.cidr.is_member(ip)
    }

    fn new_unchecked(cidr: Ipv4Cidr) -> Self {
        VpcSubnet4 { cidr }
    }

    // NEXT Instead of all this, have an Ipv4Cidr that take an IPv4
    // and net prefiex, verifieds it's not > 32, and then has getters
    // for masked IP and prefix, then this function will first create
    // that value from these two values and then check if the Ipv4Cidr
    // sits in one of the allowed blocks.
    pub fn new(cidr: Ipv4Cidr) -> result::Result<Self, IpError> {
        let ip = cidr.get_ip();
        let prefix = cidr.get_net_prefix();

        match ip.into() {
            (10, _, _, _) => {
                if prefix < 8 || prefix > OXIDE_MIN_IP4_BLOCK {
                    return Err(IpError::BadNetPrefix(prefix));
                }

                Ok(Self::new_unchecked(cidr))
            }

            (172, 16, _, _) => {
                if prefix < 12 || prefix > OXIDE_MIN_IP4_BLOCK {
                    return Err(IpError::BadNetPrefix(prefix));
                }

                Ok(Self::new_unchecked(cidr))
            }

            (192, 168, _, _) => {
                if prefix < 16 || prefix > OXIDE_MIN_IP4_BLOCK {
                    return Err(IpError::BadNetPrefix(prefix));
                }

                Ok(Self::new_unchecked(cidr))
            }

            _ => {
                return Err(IpError::Ipv4NonPrivateNetwork(ip));
            }
        }
    }
}

impl FromStr for VpcSubnet4 {
    type Err = IpError;

    fn from_str(val: &str) -> result::Result<Self, Self::Err> {
        let cidr = val.parse::<Ipv4Cidr>()?;
        VpcSubnet4::new(cidr)
    }
}

#[derive(Deserialize, Serialize)]
pub struct SetVpcSubnet4Req {
    pub vpc_sub_cidr: String,
}

#[derive(Deserialize, Serialize)]
pub struct SetVpcSubnet4Resp {
    pub resp: result::Result<(), String>,
}

#[test]
fn bad_subnet() {
    assert_eq!(
        "12.0.0.0/8".parse::<VpcSubnet4>(),
        Err(IpError::Ipv4NonPrivateNetwork("12.0.0.0".parse().unwrap()))
    );

    assert_eq!(
        "192.168.2.9/27".parse::<VpcSubnet4>(),
        Err(IpError::BadNetPrefix(27))
    );

    assert_eq!(
        "10.0.0.0/7".parse::<VpcSubnet4>(),
        Err(IpError::BadNetPrefix(7))
    );
}

#[test]
fn good_subnet() {
    assert_eq!(
        "192.168.13.0/24".parse::<VpcSubnet4>(),
        Ok(VpcSubnet4 { cidr: "192.168.13.0/24".parse().unwrap() })
    );

    assert_eq!(
        "10.64.0.0/12".parse::<VpcSubnet4>(),
        Ok(VpcSubnet4 { cidr: "10.64.0.0/12".parse().unwrap() })
    );

    assert_eq!(
        // 0000_1010_0100_1000 => 0000_1010_0100_0000
        "10.72.0.0/12".parse::<VpcSubnet4>(),
        Ok(VpcSubnet4 { cidr: "10.64.0.0/12".parse().unwrap() })
    );
}

#[test]
fn subnet_membership() {
    let ip1 = "192.168.10.99".parse().unwrap();
    let ip2 = "192.168.11.99".parse().unwrap();
    let sub = "192.168.10.0/24".parse::<VpcSubnet4>().unwrap();
    assert!(sub.is_member(ip1));
    assert!(!sub.is_member(ip2));
}