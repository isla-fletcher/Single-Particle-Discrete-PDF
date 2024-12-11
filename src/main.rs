use std::{array::from_fn, f64::consts::{E, PI}, fs::File, thread};

use noise::{NoiseFn, Perlin};
use png_encode_mini::{write_rgba_from_u32, write_rgba_from_u8};


// returns a full sample of perlin noise to use as our hypothetical function to minimize.
fn sample_noise<const W : usize, const H : usize>(dx : f64, dy : f64) -> [[f64; W]; H] {
    let perlin = Perlin::new(513432);
    from_fn(|y| from_fn(|x| {
        perlin.get([x as f64 * dx, y as f64 * dy])
    }))
}

// gets P(x == x) for X~Normal(u, var*I) in N dimensions.
// where x,u are in R^N
// and var is a scalar.
fn pdf_normal<const N : usize>(x : [f64;N], u : [f64;N], var : f64) -> f64 {
    (
        -0.5 * (
            from_fn::<f64, N, _>(|i| x[i] - u[i])
            .iter()
            .fold(0f64, |acc, v| acc + v*v)/(var)
            + (N as f64 *(var * 2.0 * PI).ln())
        )
    ).exp()// * o.powi(-(N as i32))
} 

struct Color {
    r : f64,
    g : f64,
    b : f64
}

fn to_rgba_u32(full: &Box<[Color]>) -> Box<[u32]> {
    full.iter().map(|i| {
        0xFF000000 |
        ((i.r as u32).clamp(0, 0xFF)      ) |
        ((i.g as u32).clamp(0, 0xFF) <<  8) |
        ((i.b as u32).clamp(0, 0xFF) << 16)
    }).collect()
}

fn pdf_rejection_2d(dx : f64, dy : f64, u: [f64;2], var : f64, func_to_min: &dyn Fn([f64;2]) -> f64) -> Box<[f64]> {
    let reject = (func_to_min)(u);
    (0..(WIDTH * HEIGHT)).into_iter().map(|i| {
        let (x,y) = (i % WIDTH, i / WIDTH);
        let x = [x as f64 * dx, y as f64 * dy];
        if (func_to_min)(x) > reject {
            0.0
        } else {
            pdf_normal(x, u, var)
        }
    }).collect()
}

fn pdf_iteration(dx : f64, dy: f64, m_k : Box<[f64]>, p_w_k : &dyn Fn([f64;2], [f64;2]) -> f64) -> Box<[f64]> {

    // use to ensure that the integral over each pdf == 1
    let pdf_scalars : Box<[f64]> = (0..(WIDTH * HEIGHT)).into_iter().map(|i| {
        let (ux, uy) = ((i % WIDTH) as f64 * dx, (i / WIDTH) as f64 * dy);
        (0..(WIDTH * HEIGHT)).into_iter().fold(0.0, |acc, i| {
            let (x,y) = ((i % WIDTH) as f64 * dx, (i / WIDTH) as f64 * dy);
            let value = (p_w_k)([ux,uy], [x,y]);
            value + acc
        })
    }).collect();

    (0..(WIDTH * HEIGHT)).into_iter().map(|i| {
        let (x,y) = ((i % WIDTH) as f64 * dx, (i / WIDTH) as f64 * dy);
        println!("Calculating one pixel! {}/{}",i, WIDTH * HEIGHT);
        (0..(WIDTH * HEIGHT)).into_iter().fold(0.0, |running, index| {
            let (ux, uy) = ((index % WIDTH) as f64 * dx, (index / WIDTH) as f64 * dy);
            running + (m_k[index] * (p_w_k)([ux,uy], [x,y]) / pdf_scalars[index] )
        })
    }).collect()
}

const WIDTH : usize = 100;
const HEIGHT : usize = 100;

fn run() {
    let noise = Perlin::new(342353);
    let noise2 = Perlin::new(329724);
    let noise3 = Perlin::new(262423);
    let sample_noise = |input| {
        let [x,y] = input;
        noise.get([x,y]) + 0.5 * noise2.get([x * 2.0, y * 2.0]) + 0.25 * noise3.get([x * 4.0, y * 4.0]) 
    };
    

    const dx : f64 = 0.025;
    const dy : f64 = 0.025;
    const u : [f64;2] = [WIDTH as f64 * 0.5 * dx, HEIGHT as f64 * 0.5 * dy];
    const var : f64 = 0.10;

    let sample_basic_pdf = |x| pdf_normal(x, u, var);

    let mut dist = pdf_rejection_2d(
        dx, dy, 
        u, var, &sample_noise);

    
    let mut iteration : Box<[Color]> = (0..(WIDTH * HEIGHT)).into_iter().map(
        |i| {
            let (x,y) = ((i % WIDTH) as f64 * dx, (i / WIDTH) as f64 * dy);
            let value = ((sample_noise)([x,y]) + 1.75) * 255.0 / 3.5;
            let contour = 
                (value -  32.0).abs() < 2.0 || 
                (value -  64.0).abs() < 2.0 || 
                (value - 128.0).abs() < 2.0 || 
                (value - 192.0).abs() < 2.0;
            Color {
                r: 0.0,// if contour { 125.0 } else { 0.0 },
                g: dist[i] * 255.0,
                b: value
            }
        }
    ).collect();

    let iter_access = &mut iteration;

    for i in 0..10 {

        let max_probability = dist.iter().fold(0.0, |r, next| f64::max(r,*next));

        for i in 0..dist.len() {
            dist[i] /= max_probability;
        }

        for i in 0..iter_access.len() {
            iter_access[i].g = dist[i] * 255.0;
        }

        let mut file = File::create(format!("normal_img_{}.png", i)).expect("Failed to open file writing!");
    
        write_rgba_from_u32(&mut file, to_rgba_u32(iter_access).as_ref(), WIDTH as _, HEIGHT as _);
        println!("Finished image output! ({i})");
    
        dist = pdf_iteration(dx, dy, dist, &|u_, x| {
            let x_i = (x[0] / dx) as usize + WIDTH * (x[1] / dy) as usize;
            let u_i = (u_[0] / dx) as usize + WIDTH * (u_[1] / dy) as usize;
            if sample_noise(x) > sample_noise(u_) {
            //if iter_access[x_i].b > iter_access[u_i].b {
                0.0
            } else {
                pdf_normal(x, u_, var)
            }
        });

    }
}

fn main() {
    thread::Builder::new()
        .stack_size(1 << 24)
        .spawn(run)
        .unwrap().join().unwrap();
}