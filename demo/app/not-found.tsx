import Link from 'next/link';
import { Header, Footer } from '@/components/site';
export default function NotFound(){return <><Header/><main id="main" className="wrap empty-state"><p className="eyebrow">404</p><h1>Page <em>not found.</em></h1><Link className="button primary" href="/calendar">Go to the calendar ↗</Link></main><Footer/></>}
