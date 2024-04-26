// export App1
// export App2
// export App3

// expected output:
// App
//     -> buttons
//         -> counter
//     -> links
//         -> react
//         -> vite
//     -> paragraphs
//         -> read-the-docs
//         -> test-hmr
//     -> titles
//         -> big-title
//

import './App.css'
import {
  Counter,
  ReactDocLink,
  ViteDocLink,
  ReadTheDocs,
  TestHMR,
  BigTitle,
} from './components'
import * as Components from 'components'

// App1: [
//   [Counter, "./components"],
//   [ReactDocLink, "./components"],
//   [ViteDocLink, "./components"],
//   [ReadTheDocs, "./components"],
//   [TestHMR, "./components"],
//   [BigTitle, "./components"],
// ]
export function App1() {
  return (
    <>
      <div>
        <ViteDocLink />
        <ReactDocLink />
      </div>
      <BigTitle />
      <div className="card">
        <Counter />
        <TestHMR />
      </div>
      <ReadTheDocs />
    </>
  )
}

// App2: [
//   [Counter, "./components"],
//   [ReactDocLink, "./components"],
//   [ViteDocLink, "./components"],
//   [ReadTheDocs, "./components"],
//   [TestHMR, "./components"],
//   [BigTitle, "./components"],
// ]
export function App2() {
  return (
    <>
      <div>
        <Components.ViteDocLink />
        <Components.ReactDocLink />
      </div>
      <Components.BigTitle />
      <div className="card">
        <Components.Counter />
        <Components.TestHMR />
      </div>
      <Components.ReadTheDocs />
    </>
  )
}

// App3 depends on everything components/index.ts exports
// because it uses Components directly instead of Component.SomeProperty
//
// App3: [
//   [UnusedButton, "./components"],
//   [ReactDocLink, "./components"],
//   [ViteDocLink, "./components"],
//   [UnusedLink, "./components"],
//   [ReadTheDocs, "./components"],
//   [TestHMR, "./components"],
//   [UnusedParagraph, "./components"],
//   [BigTitle, "./components"],
//   [UnusedTitle, "./components"],
//   [UnusedAvatar, "./components"],
//   [UnusedBanner, "./components"],
// ]
export function App3() {
  return <App3Inner Components={Components} />
}

// AppInner depends on nothing since it doesn't use any imported symbol
// or any local declared symbol.
//
// App3Inner: []
function App3Inner({
  Components,
}: {
  Components: {
    BigTitle: () => React.ReactNode
    Counter: () => React.ReactNode
    ReactDocLink: () => React.ReactNode
    ReadTheDocs: () => React.ReactNode
    TestHMR: () => React.ReactNode
    ViteDocLink: () => React.ReactNode
  }
}) {
  return (
    <>
      <div>
        <Components.ViteDocLink />
        <Components.ReactDocLink />
      </div>
      <Components.BigTitle />
      <div className="card">
        <Components.Counter />
        <Components.TestHMR />
      </div>
      <Components.ReadTheDocs />
    </>
  )
}
