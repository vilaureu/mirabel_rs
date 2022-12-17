use skia_safe::{
    gpu::{
        gl::{Format, FramebufferInfo},
        BackendRenderTarget, DirectContext, SurfaceOrigin,
    },
    ColorType, Surface,
};

pub fn create_surface(width: i32, height: i32) -> Surface {
    let mut gr_context = DirectContext::new_gl(None, None).unwrap();

    let mut fboid: gl::types::GLint = 0;
    let mut samples: gl::types::GLint = 0;
    let mut stencil_bits: gl::types::GLint = 0;
    unsafe {
        gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid);
        gl::GetIntegerv(gl::SAMPLES, &mut samples);
        gl::GetFramebufferAttachmentParameteriv(
            gl::DRAW_FRAMEBUFFER,
            gl::STENCIL,
            gl::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE,
            &mut stencil_bits,
        );
    }

    let fb_info = FramebufferInfo {
        fboid: fboid.try_into().expect("frame buffer id conversion"),
        format: Format::RGBA8.into(),
    };
    let backend_render_target = BackendRenderTarget::new_gl(
        (width, height),
        Some(samples.try_into().expect("samples conversion")),
        stencil_bits.try_into().expect("stencil bits conversion"),
        fb_info,
    );
    Surface::from_backend_render_target(
        &mut gr_context,
        &backend_render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .unwrap()
}

mod gl {
    #![allow(clippy::unused_unit)]
    #![allow(clippy::upper_case_acronyms)]

    include!(concat!(env!("OUT_DIR"), "/gl.rs"));
}
