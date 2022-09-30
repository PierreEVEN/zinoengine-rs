use enumflags2::*;
use std::ffi::{CString, NulError};
use std::{mem, slice};
use ze_freetype_sys::*;

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum Error {
    // ze_freetype errors
    CannotConvertToCString = 22010,

    // freetype errors
    Success = 0,
    CannotOpenResource = 1,
    UnknownFileFormat = 2,
    InvalidFileFormat = 3,
    InvalidVersion = 4,
    LowerModuleVersion = 5,
    InvalidArgument = 6,
    UnimplementedFeature = 7,
    InvalidTable = 8,
    InvalidOffset = 9,
    ArrayTooLarge = 10,
    MissingModule = 11,
    MissingProperty = 12,
    InvalidGlyphIndex = 16,
    InvalidCharacterCode = 17,
    InvalidGlyphFormat = 18,
    CannotRenderGlyph = 19,
    InvalidOutline = 20,
    InvalidComposite = 21,
    TooManyHints = 22,
    InvalidPixelSize = 23,
    InvalidSVGDocument = 24,
    InvalidHandle = 32,
    InvalidLibraryHandle = 33,
    InvalidDriverHandle = 34,
    InvalidFaceHandle = 35,
    InvalidSizeHandle = 36,
    InvalidSlotHandle = 37,
    InvalidCharMapHandle = 38,
    InvalidCacheHandle = 39,
    InvalidStreamHandle = 40,
    TooManyDrivers = 48,
    TooManyExtensions = 49,
    OutOfMemory = 64,
    UnlistedObject = 65,
    CannotOpenStream = 81,
    InvalidStreamSeek = 82,
    InvalidStreamSkip = 83,
    InvalidStreamRead = 84,
    InvalidStreamOperation = 85,
    InvalidFrameOperation = 86,
    NestedFrameAccess = 87,
    InvalidFrameRead = 88,
    RasterUninitialized = 96,
    RasterCorrupted = 97,
    RasterOverflow = 98,
    RasterNegativeHeight = 99,
    TooManyCaches = 112,
    InvalidOpcode = 128,
    TooFewArguments = 129,
    StackOverflow = 130,
    CodeOverflow = 131,
    BadArgument = 132,
    DivideByZero = 133,
    InvalidReference = 134,
    DebugOpCode = 135,
    ENDFInExecStream = 136,
    NestedDEFS = 137,
    InvalidCodeRange = 138,
    ExecutionTooLong = 139,
    TooManyFunctionDefs = 140,
    TooManyInstructionDefs = 141,
    TableMissing = 142,
    HorizHeaderMissing = 143,
    LocationsMissing = 144,
    NameTableMissing = 145,
    CMapTableMissing = 146,
    HmtxTableMissing = 147,
    PostTableMissing = 148,
    InvalidHorizMetrics = 149,
    InvalidCharMapFormat = 150,
    InvalidPPem = 151,
    InvalidVertMetrics = 152,
    CouldNotFindContext = 153,
    InvalidPostTableFormat = 154,
    InvalidPostTable = 155,
    DEFInGlyfBytecode = 156,
    MissingBitmap = 157,
    MissingSVGHooks = 158,
    SyntaxError = 160,
    StackUnderflow = 161,
    Ignore = 162,
    NoUnicodeGlyphName = 163,
    GlyphTooBig = 164,
    MissingStartfontField = 176,
    MissingFontField = 177,
    MissingSizeField = 178,
    MissingFontboundingboxField = 179,
    MissingCharsField = 180,
    MissingStartcharField = 181,
    MissingEncodingField = 182,
    MissingBbxField = 183,
    BbxTooBig = 184,
    CorruptedFontHeader = 185,
    CorruptedFontGlyphs = 186,
    Max = 187,
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::CannotConvertToCString
    }
}

impl From<FT_Error> for Error {
    fn from(error: FT_Error) -> Self {
        assert!(error < Error::Max as FT_Error);
        unsafe { mem::transmute(error) }
    }
}

/// A handle to a FreeType library instance.
/// Each ‘library’ is completely independent from the others; it is the ‘root’ of a set of objects like fonts, faces, sizes, etc.
pub struct Library {
    library: FT_Library,
}

unsafe impl Send for Library {}

impl Library {
    pub fn new() -> Result<Self, Error> {
        unsafe {
            let mut library = std::ptr::null_mut();
            let error = FT_Init_FreeType(&mut library);
            if error == FT_Err_Ok as _ {
                Ok(Self { library })
            } else {
                Err(error.into())
            }
        }
    }

    pub fn new_face(&mut self, path: &str, face_index: i32) -> Result<Face, Error> {
        unsafe {
            let mut face = std::ptr::null_mut();
            let c_file = CString::new(path)?;
            let error = FT_New_Face(self.library, c_file.as_ptr(), face_index as _, &mut face);
            if error == FT_Err_Ok as _ {
                Ok(Face { face })
            } else {
                Err(error.into())
            }
        }
    }

    pub fn new_memory_face(&mut self, data: &[u8], face_index: i32) -> Result<Face, Error> {
        unsafe {
            let mut face = std::ptr::null_mut();
            let error = FT_New_Memory_Face(
                self.library,
                data.as_ptr(),
                data.len().try_into().unwrap(),
                face_index as _,
                &mut face,
            );
            if error == FT_Err_Ok as _ {
                Ok(Face { face })
            } else {
                Err(mem::transmute(error))
            }
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FT_Done_FreeType(self.library);
        }
    }
}

#[bitflags]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum LoadFlagBits {
    NoScale = FT_LOAD_NO_SCALE,
    NoHinting = FT_LOAD_NO_HINTING,
    Render = FT_LOAD_RENDER,
    NoBitmap = FT_LOAD_NO_BITMAP,
    VerticalLayout = FT_LOAD_VERTICAL_LAYOUT,
    ForceAutoHint = FT_LOAD_FORCE_AUTOHINT,
    CropBitmap = FT_LOAD_CROP_BITMAP,
    Pedantic = FT_LOAD_PEDANTIC,
    IgnoreGlobalAdvanceWidth = FT_LOAD_IGNORE_GLOBAL_ADVANCE_WIDTH,
    NoRecurse = FT_LOAD_NO_RECURSE,
    IgnoreTransform = FT_LOAD_IGNORE_TRANSFORM,
    Monochrome = FT_LOAD_MONOCHROME,
    LinearDesign = FT_LOAD_LINEAR_DESIGN,
    SBitsOnly = FT_LOAD_SBITS_ONLY,
    NoAutoHint = FT_LOAD_NO_AUTOHINT,
    Color = FT_LOAD_COLOR,
    ComputeMetrics = FT_LOAD_COMPUTE_METRICS,
    BitmapMetricsOnly = FT_LOAD_BITMAP_METRICS_ONLY,
    AdvanceOnly = FT_LOAD_ADVANCE_ONLY,
    SvgOnly = FT_LOAD_SVG_ONLY,
}
pub type LoadFlags = BitFlags<LoadFlagBits>;

#[cfg(windows)]
#[repr(i32)]
pub enum RenderMode {
    Normal = FT_Render_Mode__FT_RENDER_MODE_NORMAL,
    Light = FT_Render_Mode__FT_RENDER_MODE_LIGHT,
    Mono = FT_Render_Mode__FT_RENDER_MODE_MONO,
    Lcd = FT_Render_Mode__FT_RENDER_MODE_LCD,
    LcdV = FT_Render_Mode__FT_RENDER_MODE_LCD_V,
    Sdf = FT_Render_Mode__FT_RENDER_MODE_SDF,
}

#[cfg(not(windows))]
#[repr(u32)]
pub enum RenderMode {
    Normal = FT_Render_Mode__FT_RENDER_MODE_NORMAL,
    Light = FT_Render_Mode__FT_RENDER_MODE_LIGHT,
    Mono = FT_Render_Mode__FT_RENDER_MODE_MONO,
    Lcd = FT_Render_Mode__FT_RENDER_MODE_LCD,
    LcdV = FT_Render_Mode__FT_RENDER_MODE_LCD_V,
    Sdf = FT_Render_Mode__FT_RENDER_MODE_SDF,
}

/// A handle to a typographic face object. A face object models a given typeface, in a given style.
pub struct Face {
    face: FT_Face,
}

unsafe impl Send for Face {}

impl Face {
    pub fn load_glyph(&self, glyph_index: u32, load_flags: LoadFlags) -> Result<(), Error> {
        unsafe { FT_Set_Pixel_Sizes(self.face, 0, 48) };
        let error = unsafe { FT_Load_Glyph(self.face, glyph_index, load_flags.bits() as i32) };
        if error == FT_Err_Ok as _ {
            Ok(())
        } else {
            Err(error.into())
        }
    }

    pub fn render_glyph(&self, render_mode: RenderMode) -> Result<GlyphSlot, Error> {
        let error = unsafe { FT_Render_Glyph((*self.face).glyph, render_mode as _) };
        if error == FT_Err_Ok as _ {
            Ok(self.glyph_slot())
        } else {
            Err(error.into())
        }
    }

    pub fn char_index(&self, charcode: u32) -> u32 {
        unsafe { FT_Get_Char_Index(self.face, charcode as _) as _ }
    }

    pub fn glyph_slot(&self) -> GlyphSlot {
        GlyphSlot(unsafe { (*self.face).glyph })
    }

    pub fn num_glyphs(&self) -> u32 {
        unsafe { (*self.face).num_glyphs as _ }
    }

    pub fn num_fixed_sizes(&self) -> i32 {
        unsafe { (*self.face).num_fixed_sizes }
    }

    pub fn num_charmaps(&self) -> i32 {
        unsafe { (*self.face).num_charmaps }
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe {
            FT_Done_Face(self.face);
        }
    }
}

#[derive(Debug)]
pub struct Bitmap(FT_Bitmap);

impl Bitmap {
    pub fn width(&self) -> u32 {
        self.0.width
    }

    pub fn pitch(&self) -> i32 {
        self.0.pitch
    }

    pub fn rows(&self) -> u32 {
        self.0.rows
    }

    pub fn data(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.0.buffer, (self.rows() as i32 * self.pitch()) as usize)
        }
    }
}

pub struct GlyphSlot(FT_GlyphSlot);

impl GlyphSlot {
    pub fn bitmap(&self) -> Bitmap {
        Bitmap(unsafe { (*self.0).bitmap })
    }
    pub fn metrics(&self) -> FT_Glyph_Metrics {
        unsafe { (*self.0).metrics }
    }
}
