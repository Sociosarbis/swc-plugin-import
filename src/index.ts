import { ImportDeclaration, Module } from "@swc/core"
import Visitor from "@swc/core/Visitor"


type CustomStylePath = (name: string) => string | false

type CustomName = (name: string) => string

type Options = {
    libraryName: string,
    style?: boolean | 'css' | CustomStylePath,
    styleLibraryDirectory?: string
    camel2DashComponentName?: boolean
    customName?: CustomName | string
}

const isEmptyImportDecl = (importDecl: ImportDeclaration) => !importDecl.specifiers.length

export default class UILibImporter extends Visitor {
    options: Options
    importItems: ImportDeclaration[] = []
    constructor(options: Options) {
        super()
        this.options = options
    }

    visitModule(m: Module) {
        m = super.visitModule(m)
        for (let i = m.body.length - 1; i >= 0; i--) {
            const item = m.body[i]
            if (item.type === 'ImportDeclaration') {
                const importDecl = item as ImportDeclaration
                if (importDecl.source.value === this.options.libraryName &&
                    isEmptyImportDecl(importDecl)) {
                    m.body.splice(i, 1)
                }
            }
        }
        m.body.unshift(...this.importItems)
        return m
    }

    visitImportDeclaration(n: ImportDeclaration) {
        if (n.source.value === this.options.libraryName) {
            for (let i = n.specifiers.length - 1; i >= 0; i--) {
                const specifier = n.specifiers[i]
                if (specifier.type === 'ImportSpecifier' && specifier.imported?.value) {
                    const componentPath = this.generateComponentPath(specifier.imported.value)
                    this.importItems.push({
                        type: 'ImportDeclaration',
                        specifiers: [{
                            type: 'ImportDefaultSpecifier',
                            local: specifier.local,
                            span: specifier.span
                        }],
                        span: specifier.span,
                        source: {
                            type: 'StringLiteral',
                            span: n.source.span,
                            value: componentPath,
                            has_escape: n.source.has_escape
                        }
                    })
                    if (this.options.style || this.options.styleLibraryDirectory) {
                        const stylePath = this.generateStyleSource(specifier.imported.value)
                        if (stylePath) {
                            this.importItems.push({
                                type: 'ImportDeclaration',
                                specifiers: [],
                                span: specifier.span,
                                source: {
                                    type: 'StringLiteral',
                                    span: n.source.span,
                                    value: stylePath,
                                    has_escape: n.source.has_escape
                                }
                            })
                        }
                    }
                    n.specifiers.splice(i, 1)
                }
            }
            return n
        }
        return super.visitImportDeclaration(n)
    }

    generateComponentPath(source: string) {
        if (this.options.customName) {
            if (typeof this.options.customName === 'function') {
                return this.options.customName(source)
            } else {
                return (require(this.options.customName).customName as CustomName)(source)
            }
        }
        return `${this.options.libraryName}/lib/${this.generateComponentName(source)}`
    }

    generateStyleSource(source: string) {
        if (typeof this.options.style === 'function') {
            return (this.options.style as CustomStylePath).call(null, source)
        }
        if (this.options.styleLibraryDirectory) {
            return `${this.options.libraryName}/${this.options.styleLibraryDirectory}/${this.generateComponentName(source)}`
        } else if (this.options.style){
            const s = this.options.style ? 'style' : 'style/css'
            return `${this.generateComponentPath(source)}/${s}`
        }
    }

    generateComponentName(source: string) {
        return this.options.camel2DashComponentName ?
            source.replace(/(?<=[a-z])([A-Z])/g, (m) => `-${m.toLowerCase()}`).replace(/^[A-Z]/, (m) => m.toLowerCase()) :
            source
    }
}