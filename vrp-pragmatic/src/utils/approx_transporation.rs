#[cfg(test)]
#[path = "../../tests/unit/utils/approx_transportation_test.rs"]
mod approx_transportation_test;

use crate::json::Location;

/// Gets approximated durations and distances.
pub fn get_approx_transportation(locations: &Vec<Location>, speed: f64) -> (Vec<f64>, Vec<f64>) {
    locations
        .iter()
        .flat_map(|l1| {
            locations.iter().map(move |l2| {
                let distance = get_distance(l1, l2);
                let duration = distance / speed;
                (duration, distance)
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .unzip()
}

/// Gets distance between two points using haversine formula.
fn get_distance(p1: &Location, p2: &Location) -> f64 {
    let d_lat = degree_rad(p1.lat - p2.lat);
    let d_lng = degree_rad(p1.lng - p2.lng);

    let lat1 = degree_rad(p1.lat);
    let lat2 = degree_rad(p2.lat);

    let a =
        (d_lat / 2.).sin() * (d_lat / 2.).sin() + (d_lng / 2.).sin() * (d_lng / 2.).sin() * (lat1).cos() * (lat2).cos();
    let c = 2. * a.sqrt().atan2((1. - a).sqrt());

    let radius = wgs84_earth_radius(d_lat);

    radius * c
}

/// Converts degrees to radians.
#[inline(always)]
fn degree_rad(degrees: f64) -> f64 {
    std::f64::consts::PI * degrees / 180.
}

#[inline(always)]
fn wgs84_earth_radius(lat: f64) -> f64 {
    // Semi-axes of WGS-84 geoidal reference
    const WGS84_A: f64 = 6378137.0; // Major semiaxis [m]
    const WGS84_B: f64 = 6356752.3; // Minor semiaxis [m]

    // http://en.wikipedia.org/wiki/Earth_radius
    let an = WGS84_A * WGS84_A * lat.cos();
    let bn = WGS84_B * WGS84_B * lat.sin();
    let ad = WGS84_A * lat.cos();
    let bd = WGS84_B * lat.sin();

    ((an * an + bn * bn) / (ad * ad + bd * bd)).sqrt()
}
