import { transformSync, parseSync } from '@swc/core'
import UILibImporter from './index'
import { readFileSync } from 'fs'
import { join } from 'path'

const FIXTURE_BASE_DIR = join(__dirname, '__fixtures__')

test('basic transform', () => {
    const { code } = transformSync("import { MessageBox, Slider, Loading } from 'element-ui'", {
        plugin: (p) => new UILibImporter({
            libraryName: 'element-ui'
        }).visitProgram(p)
    })
    expect(code).toEqual(readFileSync(join(FIXTURE_BASE_DIR, 'basic_transform.js'), 'utf-8'))
})

test('transform with styleLibraryDirectory', () => {
    const { code } = transformSync("import { MessageBox, Slider, Loading } from 'element-ui'", {
        plugin: (p) => new UILibImporter({
            libraryName: 'element-ui',
            styleLibraryDirectory: 'lib/theme-chalk'
        }).visitProgram(p)
    })
    expect(code).toEqual(readFileSync(join(FIXTURE_BASE_DIR, 'transform_with_style_library_directory.js'), 'utf-8'))
})

test('script transform', () => {
    const { code } = transformSync("const { MessageBox, Slider, Loading } = require('element-ui')", {
        isModule: false,
        plugin: (p) => {
           return  new UILibImporter({
            libraryName: 'element-ui'
        }).visitProgram(p)
    }
    })
    expect(code).toEqual(readFileSync(join(FIXTURE_BASE_DIR, 'script_transform.js'), 'utf-8'))
})