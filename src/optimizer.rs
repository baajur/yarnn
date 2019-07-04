use crate::tensor::TensorShape;
use crate::backend::Backend;


pub trait OptimizerContext {
    fn new<S: Into<TensorShape>>(shape: S) -> Self;
}

pub trait Optimizer<N, B: Backend<N>> {
    type Context: OptimizerContext;

    fn update_gradients(&self, backend: &B, ctx: &mut Self::Context, grads: &mut B::Tensor, params: &B::Tensor);
}

impl <'a, N, B: Backend<N>, O: Optimizer<N, B>> Optimizer<N, B> for &'a O {
    type Context = O::Context;

    #[inline]
    fn update_gradients(&self, backend: &B, ctx: &mut Self::Context, grads: &mut B::Tensor, params: &B::Tensor) {
        (**self).update_gradients(backend, ctx, grads, params)
    }
}

pub trait Optimizable<N, B: Backend<N>, O: Optimizer<N, B>> {
    fn calc_gradients(&mut self, backend: &B, inputs: &B::Tensor, deltas: &B::Tensor);
    fn optimize(&mut self, backend: &B, optimizer: &O);
}