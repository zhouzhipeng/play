#!/usr/bin/env swift
import Cocoa

let size: CGFloat = 1024
let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon_1024.png"

let img = NSImage(size: NSMakeSize(size, size))
img.lockFocus()

guard let ctx = NSGraphicsContext.current?.cgContext else {
    fatalError("No context")
}

// ── Background: macOS-style rounded rect with dark gradient ──
let rect = CGRect(x: 0, y: 0, width: size, height: size)
let radius: CGFloat = size * 0.22
let bgPath = CGMutablePath()
bgPath.addRoundedRect(in: rect, cornerWidth: radius, cornerHeight: radius)
ctx.addPath(bgPath)
ctx.clip()

let cs = CGColorSpaceCreateDeviceRGB()
let bgGrad = CGGradient(colorsSpace: cs, colors: [
    CGColor(srgbRed: 0.07, green: 0.08, blue: 0.16, alpha: 1.0),
    CGColor(srgbRed: 0.10, green: 0.14, blue: 0.26, alpha: 1.0),
] as CFArray, locations: [0.0, 1.0])!
ctx.drawLinearGradient(bgGrad, start: CGPoint(x: 0, y: size), end: CGPoint(x: size, y: 0), options: [])

// ── Subtle grid lines for depth ──
ctx.setStrokeColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.03))
ctx.setLineWidth(1.0)
let step: CGFloat = 48
var y: CGFloat = step
while y < size {
    ctx.move(to: CGPoint(x: 0, y: y))
    ctx.addLine(to: CGPoint(x: size, y: y))
    y += step
}
ctx.strokePath()

// ── Helper: draw text centered ──
func drawText(_ text: String, at point: NSPoint, font: NSFont, color: NSColor) {
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: color,
    ]
    let str = NSAttributedString(string: text, attributes: attrs)
    let line = CTLineCreateWithAttributedString(str)
    let bounds = CTLineGetBoundsWithOptions(line, .useOpticalBounds)
    ctx.saveGState()
    ctx.textPosition = CGPoint(x: point.x - bounds.width / 2, y: point.y - bounds.height / 2)
    CTLineDraw(line, ctx)
    ctx.restoreGState()
}

// ── Large curly braces "{ }" ──
let braceFont = NSFont.systemFont(ofSize: 420, weight: .thin)
let teal = NSColor(srgbRed: 0.31, green: 0.79, blue: 0.69, alpha: 1.0)

// Left brace
drawText("{", at: NSPoint(x: size * 0.28, y: size * 0.46), font: braceFont, color: teal)
// Right brace
drawText("}", at: NSPoint(x: size * 0.72, y: size * 0.46), font: braceFont, color: teal)

// ── Arrow ">" in the center ── execution symbol
let arrowFont = NSFont.systemFont(ofSize: 200, weight: .bold)
let orange = NSColor(srgbRed: 0.90, green: 0.44, blue: 0.33, alpha: 1.0)
drawText(">", at: NSPoint(x: size * 0.50, y: size * 0.48), font: arrowFont, color: orange)

// ── "curl" text at bottom ──
let curlFont = NSFont.monospacedSystemFont(ofSize: 90, weight: .medium)
let lightGray = NSColor(srgbRed: 0.65, green: 0.72, blue: 0.80, alpha: 0.7)
drawText("curl", at: NSPoint(x: size * 0.50, y: size * 0.14), font: curlFont, color: lightGray)

// ── Accent glow behind arrow ──
ctx.saveGState()
ctx.setBlendMode(.screen)
let glowGrad = CGGradient(colorsSpace: cs, colors: [
    CGColor(srgbRed: 0.90, green: 0.44, blue: 0.33, alpha: 0.15),
    CGColor(srgbRed: 0.90, green: 0.44, blue: 0.33, alpha: 0.0),
] as CFArray, locations: [0.0, 1.0])!
ctx.drawRadialGradient(glowGrad,
    startCenter: CGPoint(x: size * 0.50, y: size * 0.50),
    startRadius: 0,
    endCenter: CGPoint(x: size * 0.50, y: size * 0.50),
    endRadius: size * 0.25,
    options: [])
ctx.restoreGState()

img.unlockFocus()

// ── Save as PNG ──
guard let tiffData = img.tiffRepresentation,
      let rep = NSBitmapImageRep(data: tiffData),
      let png = rep.representation(using: .png, properties: [:]) else {
    fatalError("Failed to create PNG")
}
try! png.write(to: URL(fileURLWithPath: outPath))
print("Icon generated: \(outPath)")
