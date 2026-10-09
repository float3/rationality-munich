import Link from 'next/link';
import { ArrowRight, ArrowUpRight, MapPin } from 'lucide-react';
import { Header, Footer } from '@/components/site';
import { MONTHS, MonthGrid, monthsOf } from '@/components/month-grid';
import { HomeSections } from '@/components/home-sections';
import { base } from '@/lib/base';

export default function Home() {
  return <><Header /><main id="main">
    <section className="hero wrap">
      <div className="hero-copy"><p className="eyebrow"><span className="status-dot" /> FIVE GROUPS, ONE CALENDAR</p>
        <h1>Meetups in Munich for<br /><em>rationality, EA and AI safety.</em></h1>
        <p className="hero-intro">Five independent groups run discussions, dinners, workshops and talks in Munich. This page collects them and their events in one place.</p>
        <div className="actions"><Link className="button primary" href="/calendar">See the calendar <ArrowUpRight size={18} /></Link><Link className="text-link" href="/#first-visit">Your first visit <ArrowRight size={17}/></Link></div>
              </div>
      <figure className="hero-image"><img src={`${base}/images/munich.webp`} alt="The Monopteros and green lawns in Munich’s Englischer Garten" width="1600" height="979" fetchPriority="high" /><Link className="hero-event" href="/events/dine-and-discuss"><span className="eyebrow">NEXT UP · WEDNESDAY, 23 SEP</span><strong>Dine &amp; Discuss: the Hugging Face incident</strong><span>18:00 · EA Munich <ArrowUpRight size={18}/></span></Link><figcaption><MapPin size={16}/> Englischer Garten <span>MUNICH</span></figcaption></figure>
    </section>
    <div className="topic-strip"><div className="wrap"><span>RATIONALITY</span><i>✳</i><span>EFFECTIVE ALTRUISM</span><i>✳</i><span>PHILOSOPHY</span><i>✳</i><span>AI SAFETY</span><i>✳</i><span>AI POLICY</span></div></div>
    <section className="section wrap" id="events"><div className="section-heading"><div><p className="eyebrow">THE NEXT FEW WEEKS</p><h2>Coming up in Munich</h2></div><Link href="/calendar" className="text-link">All events <ArrowUpRight size={18}/></Link></div><div className="home-months">{monthsOf().map(({year,month})=><div key={`${year}-${month}`}><h3 className="month-label">{MONTHS[month]} <span>{year}</span></h3><MonthGrid year={year} month={month}/></div>)}</div><p className="section-note">Event snapshot · 22 September 2026 · All times Munich time. Check the organizer’s announcement before attending.</p></section>
    <div className="wrap"><div className="newcomer-spotlight"><div><p className="eyebrow">NO PREPARATION NEEDED</p><h3>The Estimation Game · 30 September</h3><p>Join a team and compare your best guesses. No special knowledge or preparation required.</p></div><Link className="text-link" href="/events/estimation-game">Event details <ArrowUpRight size={18}/></Link></div></div>
    <HomeSections />
  </main><Footer/></>;
}
