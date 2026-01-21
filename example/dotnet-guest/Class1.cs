using System.Runtime.InteropServices;

namespace dotnet_guest;

/// <summary>
/// Contains FFI definitions for interacting with the wasm96 host.
/// </summary>
internal static class Graphics
{
    private const string NativeLib = "__native";

    [DllImport(NativeLib, EntryPoint = "wasm96_graphics_set_size")]
    public static extern void SetSize(uint width, uint height);

    [DllImport(NativeLib, EntryPoint = "wasm96_graphics_set_color")]
    public static extern void SetColor(byte r, byte g, byte b, byte a);

    [DllImport(NativeLib, EntryPoint = "wasm96_graphics_background")]
    public static extern void Background(byte r, byte g, byte b);

    [DllImport(NativeLib, EntryPoint = "wasm96_graphics_circle")]
    public static extern void Circle(int x, int y, uint r);
}

/// <summary>
/// Main guest entry points exported to the wasm96 host via Native AOT.
/// </summary>
public static class App
{
    /// <summary>
    /// Called once on startup by the wasm96 host.
    /// </summary>
    [UnmanagedCallersOnly(EntryPoint = "setup")]
    public static void Setup()
    {
        // Initialize the screen size to 320x240.
        Graphics.SetSize(320, 240);
    }

    /// <summary>
    /// Called once per frame for logic updates.
    /// </summary>
    [UnmanagedCallersOnly(EntryPoint = "update")]
    public static void Update()
    {
        // Logic updates would go here.
    }

    /// <summary>
    /// Called once per frame for rendering.
    /// </summary>
    [UnmanagedCallersOnly(EntryPoint = "draw")]
    public static void Draw()
    {
        // Clear the screen with a dark background color.
        Graphics.Background(12, 16, 24);

        // Set the current drawing color to orange/red.
        Graphics.SetColor(255, 120, 80, 255);

        // Render a circle in the center of the screen (160, 120) with radius 50.
        Graphics.Circle(160, 120, 50);
    }

    /// <summary>
    /// Dummy entry point required by the .NET compiler for Exe output types.
    /// The actual execution starts from the exported setup/update/draw functions.
    /// </summary>
    public static void Main()
    {
    }
}
