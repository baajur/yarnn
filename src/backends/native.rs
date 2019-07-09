use crate::tensor::*;
use crate::backend::*;
use std::fmt;
extern crate openblas_src;
// use cblas::{Layout, Transpose, sgemm, saxpy};
use blas::sgemm;
use std::fmt::Write;
use rand_distr::{Normal, Distribution};


pub struct NativeTensorF32 {
    shape: TensorShape,
    ptr: Option<Box<[f32]>>
}

impl NativeTensorF32 {
    pub fn read(&self) -> &[f32] {
        self.ptr.as_ref().unwrap()
    } 

    pub fn write(&mut self) -> &mut [f32] {
        if self.ptr.is_none() {
            self.ptr = Some(vec![0.0; self.shape.size()].into_boxed_slice());
        }

        return self.ptr.as_mut().unwrap()
    }
}

impl Tensor<f32> for NativeTensorF32 {
    fn new<S: Into<TensorShape>>(shape: S) -> Self {
        NativeTensorF32 {
            shape: shape.into(),
            ptr: None,
        }
    }

    fn shape(&self) -> &TensorShape {
        &self.shape
    }

    fn resize(&mut self, shape: TensorShape) {
        self.ptr = if let Some(ptr) = self.ptr.take() {
            let size = self.shape.size();
            let raw = Box::into_raw(ptr) as *mut f32;
            let mut data = unsafe {Vec::from_raw_parts(raw, size, size)};
            data.resize(shape.size(), 0.0);

            Some(data.into_boxed_slice())
        } else {
            None
        };
        self.shape = shape;
    }
}

pub struct Native;

impl Native {
    fn fmt_tensor(&self, t: &NativeTensorF32, f: &mut String) -> fmt::Result {
        let strides = t.shape.default_strides();
        let last_idx = strides.dims - 1;
        writeln!(f, "default stridses {} {}", t.shape.default_strides(), last_idx)?;
        write!(f, "Tensor(shape={}, data=[", t.shape)?;

        for (idx, val) in t.read().iter().enumerate() {
            let is_first = idx == 0;
            let mut need_nl = false;
            let padding = 2;

            for (sidx, s) in strides.iter().enumerate() {
                if sidx != last_idx && idx % s as usize == 0 {
                    need_nl = true;
                }
            }

            if !is_first {
                write!(f, ", ")?;
            }

            if need_nl {
                write!(f, "\n{}", " ".repeat(padding))?;
            }

            write!(f, "{}", val)?;
        }

        writeln!(f, "\n])")?;

        Ok(())
    }
}

impl Backend<f32> for Native {
    type Tensor = NativeTensorF32;

    fn store_tensor_f32(&self, t: &Self::Tensor, data: &mut [f32]) {
        let size = t.shape().size();
        assert!(data.len() >= size);

        let dst = t.read();

        for i in 0 .. size {
            data[i] = dst[i] as f32;
        }
    }

    fn load_tensor_u8(&self, t: &mut Self::Tensor, data: &[u8]) {
        let size = t.shape().size();
        assert!(data.len() >= size);

        let dst = t.write();

        for i in 0 .. size {
            dst[i] = data[i] as f32;
        }
    }

    #[inline]
    fn scalar_f32(&self, val: f32) -> f32 {
        val
    }

    #[inline]
    fn fill_scalar(&self, t: &mut Self::Tensor, scalar: f32) {
        let size = t.shape().size();
        let dst = t.write();

        for i in 0 .. size {
            dst[i] = scalar;
        }
    }

    #[inline]
    fn fill_random(&self, t: &mut Self::Tensor, from: f32, to: f32) {
        let seed = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed(seed);
        let normal = Normal::new(from, to).unwrap();
        let size = t.shape().size();
        let dst = t.write();

        for i in 0 .. size {
            dst[i] = normal.sample(&mut rng);
        }
    }

    fn print_tensor(&self, t: &Self::Tensor) {
        let mut s = String::new();
        self.fmt_tensor(t, &mut s).unwrap();
        println!("{}", s);
    } 
}

impl BackendGemm<f32> for Native {
    fn matmul(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor) {
        let a_shape = a.shape();
        let b_shape = b.shape();
        let c_shape = dst.shape().clone();

        assert_eq!(a_shape.get(0), c_shape.get(0));
        assert_eq!(b_shape.get(1), c_shape.get(1));

        assert_eq!(a_shape.dims, 2);
        assert_eq!(b_shape.dims, 2);

        let m = a_shape.get(0) as i32;
        let n = b_shape.get(1) as i32;
        let k = b_shape.get(0) as i32;
        
        unsafe {
            sgemm('N' as u8, 'N' as u8,
                n, m, k, 
                1.0, 
                b.read(), n, 
                a.read(), k, 
                0.0, 
                &mut dst.write(), n);
        }
    }

    fn matmul_nt(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor) {
        let a_shape = a.shape();
        let b_shape = b.shape();
        let c_shape = dst.shape().clone();

        assert_eq!(a_shape.get(0), c_shape.get(0));
        assert_eq!(b_shape.get(0), c_shape.get(1));

        assert_eq!(a_shape.dims, 2);
        assert_eq!(b_shape.dims, 2);

        let m = a_shape.get(0) as i32;
        let n = b_shape.get(0) as i32;
        let k = b_shape.get(1) as i32;
        
        unsafe {
            sgemm('T' as u8, 'N' as u8,
                n, m, k, 
                1.0, 
                b.read(), k, 
                a.read(), k, 
                0.0, 
                &mut dst.write(), n);
        }
    }

    fn matmul_tn(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor) {
        let a_shape = a.shape();
        let b_shape = b.shape();
        let c_shape = dst.shape().clone();

        assert_eq!(a_shape.get(1), c_shape.get(0));
        assert_eq!(b_shape.get(1), c_shape.get(1));

        assert_eq!(a_shape.dims, 2);
        assert_eq!(b_shape.dims, 2);

        let m = a_shape.get(1) as i32;
        let n = b_shape.get(1) as i32;
        let k = b_shape.get(0) as i32;
        
        unsafe {
            sgemm('N' as u8, 'T' as u8,
                n, m, k, 
                1.0, 
                b.read(), n, 
                a.read(), m, 
                0.0, 
                &mut dst.write(), n);
        }
    }

    fn matmul_tt(&self, _dst: &mut Self::Tensor, _a: &Self::Tensor, _b: &Self::Tensor) {
        unimplemented!();
    }
}

impl BackendSigmoid<f32> for Native {
    fn sigmoid(&self, dst: &mut Self::Tensor, data: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(dst.shape() == data.shape());

        let data_s = &data.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = 1.0 / (1.0 + (-data_s[i]).exp());
        }
    }

    fn sigmoid_grad(&self, dst: &mut Self::Tensor, z: &Self::Tensor, d: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(dst.shape() == z.shape());
        assert!(dst.shape() == d.shape());

        let z_s = &z.read()[0 .. dst_size];
        let d_s = &d.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = (z_s[i] * (1.0 - z_s[i])) * d_s[i];
        }
    }
}

impl BackendReLu<f32> for Native {
    fn relu(&self, dst: &mut Self::Tensor, data: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(dst.shape() == data.shape());

        let data_s = &data.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            let val = if data_s[i] > 0.0 {
                data_s[i]
            } else {
                0.0
            };

            dst_s[i] = val;
        }
    }

    fn relu_grad(&self, dst: &mut Self::Tensor, z: &Self::Tensor, d: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(dst.shape() == z.shape());
        assert!(dst.shape() == d.shape());

        let z_s = &z.read()[0 .. dst_size];
        let d_s = &d.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = if z_s[i] > 0.0 {
                d_s[i]
            } else {
                0.0
            };
        }
    }
}

impl BackendBias<f32> for Native {
    fn bias_add(&self, dst: &mut Self::Tensor, biases: &Self::Tensor) {
        let biases_shape = biases.shape();
        let dst_shape = dst.shape().clone();
        let biases_size = biases_shape.get(0) as usize;
        let dst_size = dst_shape.size();
        
        assert!(dst_shape.get(dst_shape.dims - 1) as usize == biases_size);
        
        let batch_size = dst_shape.get(0) as usize;
        let biases_s = &biases.read()[0 .. biases_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        let mut inner = 1usize;

        for (idx, i) in dst_shape.as_slice().iter().enumerate() {
            if idx == 0 || idx == dst_shape.dims - 1 {
                continue;
            }

            inner *= *i as usize;
        }

        for b in 0 .. batch_size {
            for i in 0..inner {
                for l in 0..biases_size {
                    let offset = b * (inner * biases_size) + i * biases_size + l;

                    dst_s[offset] += biases_s[l];
                }
            }
        }
    }
    
    fn bias_grad(&self, dbiases: &mut Self::Tensor, deltas: &Self::Tensor) {
        let dbiases_shape = dbiases.shape();
        let deltas_shape = deltas.shape();
        let dbiases_size = dbiases_shape.get(0) as usize;
        let deltas_size = deltas_shape.size();
        
        assert!(deltas_shape.get(deltas_shape.dims - 1) as usize == dbiases_size);

        let batch_size = deltas_shape.get(0) as usize;
        let dbiases_s = &mut dbiases.write()[0 .. dbiases_size];
        let deltas_s = &deltas.read()[0 .. deltas_size];

        let mut inner = 1usize;

        for (idx, i) in deltas_shape.as_slice().iter().enumerate() {
            if idx == 0 || idx == deltas_shape.dims - 1 {
                continue;
            }

            inner *= *i as usize;
        }

        for b in 0 .. batch_size {
            for l in 0 .. dbiases_size {
                let mut bias_grad = 0.0;
                for i in 0 .. inner {
                    let offset = b * (inner * dbiases_size) + i * dbiases_size + l;
                    bias_grad += deltas_s[offset];
                }

                dbiases_s[l] = bias_grad;
            }
        }
    }
}


impl BackendScale<f32> for Native {
    fn scale(&self, dst: &mut Self::Tensor, scale: f32) {
        let dst_size = dst.shape().size();
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] *= scale;
        }
    }
}

// impl BackendScale<f32> for Native {
//     fn scale(&self, dst: &mut Self::Tensor, scale: f32) {
//         let dst_size = dst.shape().size();

//         unsafe {
//             blas::sscal(
//                 dst_size as i32,
//                 scale,
//                 dst.write(),
//                 1
//             );
//         }
//     }
// }

impl BackendMse<f32> for Native {
    fn scaled_square_diff(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor, scale: f32) {
        let a_size = a.shape().size();
        let b_size = b.shape().size();
        let dst_size = dst.shape().size();

        assert_eq!(a_size, dst_size);
        assert_eq!(b_size, dst_size);

        let a_s = &a.read()[0 .. dst_size];
        let b_s = &b.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            let diff = a_s[i] - b_s[i];

            dst_s[i] = scale * diff * diff;
        }
    }

    fn scaled_diff(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor, scale: f32) {
        let a_size = a.shape().size();
        let b_size = b.shape().size();
        let dst_size = dst.shape().size();

        assert_eq!(a_size, dst_size);
        assert_eq!(b_size, dst_size);

        let a_s = &a.read()[0 .. dst_size];
        let b_s = &b.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = scale * (a_s[i] - b_s[i]);
        }
    }
}

// impl BackendAxpy<f32> for Native {
//     default fn axpy(&self, dst: &mut Self::Tensor, scale: f32, a: &Self::Tensor) {
//         let dst_size = dst.shape().size();

//         assert!(a.shape() == dst.shape());

//         let a_s = &a.read()[0 .. dst_size];
//         let dst_s = &mut dst.write()[0 .. dst_size];

//         for i in 0 .. dst_size {
//             dst_s[i] += scale * a_s[i];
//         }
//     }
// }

impl BackendAxpy<f32> for Native {
    fn axpy(&self, dst: &mut Self::Tensor, scale: f32, x: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(x.shape() == dst.shape());

        unsafe {
            blas::saxpy(
                dst_size as i32,
                scale,
                x.read(),
                1,
                dst.write(),
                1
            );
        }
    }
}

impl BackendAxpys<f32> for Native {
    fn axpys(&self, dst: &mut Self::Tensor, scale: f32, a: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(a.shape() == dst.shape());

        let a_s = &a.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] += scale * a_s[i] * a_s[i];
        }
    }
}

impl BackendAdd<f32> for Native {
    fn add(&self, dst: &mut Self::Tensor, a: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(a.shape() == dst.shape());

        let a_s = &a.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] += a_s[i];
        }
    }
}

impl BackendSub<f32> for Native {
    fn sub(&self, dst: &mut Self::Tensor, a: &Self::Tensor, b: &Self::Tensor) {
        let a_size = a.shape().size();
        let b_size = b.shape().size();
        let dst_size = dst.shape().size();

        assert!(dst_size == a_size);
        assert!(dst_size == b_size);

        let a_s = &a.read()[0 .. dst_size];
        let b_s = &b.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = a_s[i] - b_s[i];
        }
    }
    
}

impl BackendMul<f32> for Native {
    fn mul(&self, dst: &mut Self::Tensor, a: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(a.shape() == dst.shape());

        let a_s = &a.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] *= a_s[i];
        }
    }
}


impl BackendCopy<f32> for Native {
    fn copy(&self, dst: &mut Self::Tensor, a: &Self::Tensor) {
        let size = dst.shape().size();

        assert!(a.shape() == dst.shape());

        let a_s = &a.read()[0 .. size];
        let dst_s = &mut dst.write()[0 .. size];

        for i in 0 .. size {
            dst_s[i] = a_s[i];
        }
    }
}

impl BackendMaximum<f32> for Native {
    fn maximum(&self, dst: &mut Self::Tensor, a: &Self::Tensor) {
        let dst_size = dst.shape().size();

        assert!(a.shape() == dst.shape());

        let a_s = &a.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] = f32::max(a_s[i], dst_s[i]);
        }
    }
}


impl BackendAdam<f32> for Native {
    fn adam_p(&self, dst: &mut Self::Tensor, lr: f32, moms: &Self::Tensor, vels: &Self::Tensor, eps: f32) {
        let dst_size = dst.shape().size();

        assert!(moms.shape() == dst.shape());
        assert!(vels.shape() == dst.shape());

        let moms_s = &moms.read()[0 .. dst_size];
        let vels_s = &vels.read()[0 .. dst_size];
        let dst_s = &mut dst.write()[0 .. dst_size];

        for i in 0 .. dst_size {
            dst_s[i] += lr * moms_s[i] / (vels_s[i].sqrt() + eps)
        }
    }
}

impl BackendSoftmax<f32> for Native {
    fn softmax(&self, y: &mut Self::Tensor, x: &Self::Tensor) {
        let y_shape = y.shape();
        let x_shape = x.shape();
        let size = y_shape.size();
        let axis = y_shape.last_axis() as usize;

        assert!(y_shape == x_shape);

        let x_s = &x.read()[0 .. size];
        let y_s = &mut y.write()[0 .. size];

        // copy x to y
        for i in 0..size {
            y_s[i] = x_s[i];
        }

        for i in (0..size).step_by(axis as usize) {
            assert!(i + (axis - 1) < size);

            // max(x)
            let mut max_x = std::f32::NEG_INFINITY;
            for j in 0..axis {
                let val = x_s[i + j];
                if val > max_x {
                    max_x = val;
                }
            }

            // exp(x - max(x))
            for j in 0..axis {
                let offset = i + j;
                y_s[offset] = (y_s[offset] - max_x).exp();
            }

            // 1/sum(ex)
            let mut sum = 0.0;
            for j in 0..axis {
                sum += y_s[i + j];
            }
            let rsum = 1.0 / sum;

            // ex * (1/sum(ex))
            for j in 0..axis {
                y_s[i + j] *= rsum;
            }
        }
    }
}


#[test]
fn test_softmax() {
    let bac = Native;
    let mut a = NativeTensorF32::new((3, 3));
    let mut b = NativeTensorF32::new((3, 3));

    bac.load_tensor_u8(&mut a, &[
        1,2,3,
        4,5,6,
        7,8,9,
    ]);

    bac.softmax(&mut b, &a);

    assert!(
        b.read() == &[
            0.09003057, 0.24472847, 0.66524096,  
            0.09003057, 0.24472847, 0.66524096, 
            0.09003057, 0.24472847, 0.66524096,
        ]
    );
}


#[test]
fn test_matmul() {
    let bac = Native;
    let mut a = NativeTensorF32::new((2, 3));
    let mut b = NativeTensorF32::new((3, 4));
    let mut c = NativeTensorF32::new((2, 4));

    bac.load_tensor_u8(&mut a, &[
        1,2,3,
        4,5,6
    ]);

    bac.load_tensor_u8(&mut b, &[
        1,2,3,4,
        5,6,7,8,
        9,10,11,12
    ]);

    bac.matmul(&mut c, &a, &b);

    assert!(
        c.read() == &[
            38.0,  44.0,  50.0,  56.0,
            83.0,  98.0, 113.0, 128.0,
        ]
    );
}

#[test]
fn test_matmul_nt() {
    let bac = Native;
    let mut a = NativeTensorF32::new((2, 3));
    let mut b = NativeTensorF32::new((4, 3));
    let mut c = NativeTensorF32::new((2, 4));

    bac.load_tensor_u8(&mut a, &[
        1,2,3,
        4,5,6
    ]);

    bac.load_tensor_u8(&mut b, &[
        1,5,9,
        2,6,10,
        3,7,11,
        4,8,12
    ]);

    bac.matmul_nt(&mut c, &a, &b);

    assert!(
        c.read() == &[
            38.0,  44.0,  50.0,  56.0,
            83.0,  98.0, 113.0, 128.0,
        ]
    );
}


#[test]
fn test_matmul_tn() {
    let bac = Native;
    let mut a = NativeTensorF32::new((8, 5));
    let mut b = NativeTensorF32::new((8, 3));
    let mut c = NativeTensorF32::new((5, 3));

    bac.load_tensor_u8(&mut a, &[
        0,  1,  2,  3,  4,  
        5,  6,  7,  8,  9, 
       10, 11, 12, 13, 14, 
       15, 16, 17, 18, 19, 
       20, 21, 22, 23, 24, 
       25, 26, 27, 28, 29, 
       30, 31, 32, 33, 34, 
       35, 36, 37, 38, 39
    ]);

    bac.load_tensor_u8(&mut b, &[
        0,  1,  2,  
        3,  4,  5,  
        6,  7,  8,  
        9, 10, 11,
       12, 13, 14, 
       15, 16, 17, 
       18, 19, 20, 
       21, 22, 23
    ]);

    bac.matmul_tn(&mut c, &a, &b);

    assert!(
        c.read() == &[
            2100.0, 2240.0, 2380.0,
            2184.0, 2332.0, 2480.0,
            2268.0, 2424.0, 2580.0,
            2352.0, 2516.0, 2680.0,
            2436.0, 2608.0, 2780.0
        ]
    );
}


#[test]
fn test_axpy() {
    let bac = Native;

    let mut a = NativeTensorF32::new((2, 2));
    let mut b = NativeTensorF32::new((2, 2));

    bac.load_tensor_u8(&mut a, &[1, 2, 3, 4]);
    bac.load_tensor_u8(&mut b, &[1, 2, 3, 4]);

    bac.axpy(&mut a, 2.0f32, &b);

    assert!(
        a.read() == &[3.0, 6.0, 9.0, 12.0]
    );
} 

#[test]
fn test_add() {
    let bac = Native;

    let mut a = NativeTensorF32::new((2, 2));
    let mut b = NativeTensorF32::new((2, 2));

    bac.load_tensor_u8(&mut a, &[1, 2, 3, 4]);
    bac.load_tensor_u8(&mut b, &[1, 2, 3, 4]);

    bac.add(&mut a, &b);
    
    assert!(
        a.read() == &[2.0, 4.0, 6.0, 8.0]
    );
} 