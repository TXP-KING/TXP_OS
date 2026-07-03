#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Networking packet parsing primitives.

/// Ethernet MAC address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacAddress(pub [u8; 6]);

/// Ethernet ethertype.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EtherType {
    /// IPv4 payload.
    Ipv4,
    /// ARP payload.
    Arp,
    /// IPv6 payload.
    Ipv6,
    /// Unknown ethertype.
    Unknown(u16),
}

impl EtherType {
    /// Parses an ethertype from network-order bytes.
    pub const fn from_u16(value: u16) -> Self {
        match value {
            0x0800 => Self::Ipv4,
            0x0806 => Self::Arp,
            0x86dd => Self::Ipv6,
            other => Self::Unknown(other),
        }
    }
}

/// Parsed Ethernet frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthernetFrame<'a> {
    /// Destination MAC.
    pub dst: MacAddress,
    /// Source MAC.
    pub src: MacAddress,
    /// Payload type.
    pub ethertype: EtherType,
    /// Payload bytes.
    pub payload: &'a [u8],
}

/// Packet parse errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The packet is too short for the requested header.
    Truncated,
    /// The header version is not supported by this parser.
    UnsupportedVersion,
    /// Header length field is invalid.
    InvalidHeaderLength,
}

impl<'a> EthernetFrame<'a> {
    /// Parses an Ethernet frame.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ParseError> {
        if bytes.len() < 14 {
            return Err(ParseError::Truncated);
        }

        let dst = MacAddress([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]]);
        let src = MacAddress([bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11]]);
        let ty = u16::from_be_bytes([bytes[12], bytes[13]]);

        Ok(Self {
            dst,
            src,
            ethertype: EtherType::from_u16(ty),
            payload: &bytes[14..],
        })
    }
}

/// Parsed IPv4 header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ipv4Header<'a> {
    /// Differentiated services field.
    pub dscp_ecn: u8,
    /// Total packet length from the IPv4 header.
    pub total_len: u16,
    /// Protocol number.
    pub protocol: u8,
    /// Source address.
    pub src: [u8; 4],
    /// Destination address.
    pub dst: [u8; 4],
    /// Header bytes, including options.
    pub header: &'a [u8],
    /// Payload bytes.
    pub payload: &'a [u8],
}

impl<'a> Ipv4Header<'a> {
    /// Parses an IPv4 packet.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ParseError> {
        if bytes.len() < 20 {
            return Err(ParseError::Truncated);
        }

        let version = bytes[0] >> 4;
        if version != 4 {
            return Err(ParseError::UnsupportedVersion);
        }

        let ihl = (bytes[0] & 0x0f) as usize * 4;
        if ihl < 20 {
            return Err(ParseError::InvalidHeaderLength);
        }

        if bytes.len() < ihl {
            return Err(ParseError::Truncated);
        }

        let total_len = u16::from_be_bytes([bytes[2], bytes[3]]);
        let total_len_usize = total_len as usize;
        if total_len_usize < ihl || bytes.len() < total_len_usize {
            return Err(ParseError::Truncated);
        }

        Ok(Self {
            dscp_ecn: bytes[1],
            total_len,
            protocol: bytes[9],
            src: [bytes[12], bytes[13], bytes[14], bytes[15]],
            dst: [bytes[16], bytes[17], bytes[18], bytes[19]],
            header: &bytes[..ihl],
            payload: &bytes[ihl..total_len_usize],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ethernet_frame() {
        let bytes = [1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1, 0x08, 0x00, 0xde, 0xad];
        let frame = EthernetFrame::parse(&bytes).unwrap();

        assert_eq!(frame.dst, MacAddress([1, 2, 3, 4, 5, 6]));
        assert_eq!(frame.ethertype, EtherType::Ipv4);
        assert_eq!(frame.payload, &[0xde, 0xad]);
    }

    #[test]
    fn parses_ipv4_header() {
        let packet = [
            0x45, 0, 0, 21, 0, 0, 0, 0, 64, 17, 0, 0, 192, 168, 1, 10, 192, 168, 1, 1, 99,
        ];
        let ipv4 = Ipv4Header::parse(&packet).unwrap();

        assert_eq!(ipv4.protocol, 17);
        assert_eq!(ipv4.src, [192, 168, 1, 10]);
        assert_eq!(ipv4.payload, &[99]);
    }
}
