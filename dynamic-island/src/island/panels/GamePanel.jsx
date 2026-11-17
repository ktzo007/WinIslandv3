import { Icon } from "../../shared/Icon";
import { BoostButton, Sparkline } from "../../shared/ui";
import { fmtRate, useGameMetrics } from "../hooks/useGameMetrics";

export const pingTone = (ms) => (ms == null ? "bad" : ms < 50 ? "ok" : ms < 100 ? "warn" : "bad");

export default function GamePanel({ ctx }) {
  const { s, boost } = ctx;
  const host = s.game.pingHost || "1.1.1.1";
  const g = useGameMetrics(true, host); // panel terbuka = polling aktif; unmount = berhenti
  const gpu = g.gpu;
  return (
    <div className="panel gm">
      <div className="card">
        <div className="clabel">PING · {host}</div>
        <div className={`cbig num ${pingTone(g.cur)}`}>
          {!g.hasPing ? "–" : g.cur == null ? "timeout" : g.cur}
          {g.cur != null && <small>ms</small>}
        </div>
        <div className="csub2 num">jitter {g.jitter} · loss {g.loss}%</div>
        <Sparkline data={g.ping.map((v) => v ?? 150)} max={150} />
      </div>
      <div className="card">
        {gpu ? (
          <>
            <div className="clabel" title={gpu.name}>GPU</div>
            <div className={`cbig num ${gpu.util >= 95 ? "warn" : ""}`}>{gpu.util}<small>%</small></div>
            <div className="gbar"><i style={{ transform: `scaleX(${gpu.util / 100})` }} /></div>
            <div className="csub2 num">{gpu.temp}°C · {(gpu.memUsed / 1024).toFixed(1)}/{(gpu.memTotal / 1024).toFixed(0)} GB</div>
          </>
        ) : (
          <>
            <div className="clabel">CPU · RAM</div>
            <div className="cbig num">{g.cpu != null ? Math.round(g.cpu) : "–"}<small>%</small></div>
            <div className="gbar"><i style={{ transform: `scaleX(${(g.cpu ?? 0) / 100})` }} /></div>
            <div className="csub2 num">RAM {g.ram ?? "–"}% · GPU NVIDIA saja</div>
          </>
        )}
      </div>
      <div className="gside">
        <BoostButton boost={boost} compact />
        <div className="net num">
          <span><Icon name="down" size={11} /> {fmtRate(g.net.downBps)}</span>
          <span><Icon name="up" size={11} /> {fmtRate(g.net.upBps)}</span>
        </div>
      </div>
    </div>
  );
}
