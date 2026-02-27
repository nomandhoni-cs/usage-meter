import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./NetworkUsage.css";

type TimePeriodType =
    | { type: "Today" }
    | { type: "Yesterday" }
    | { type: "ThisWeek" }
    | { type: "LastWeek" }
    | { type: "ThisMonth" }
    | { type: "LastMonth" }
    | { type: "ThisYear" }
    | { type: "LastYear" }
    | { type: "Custom"; start_date: string; end_date: string };

interface NetworkStats {
    start_date: string;
    end_date: string;
    total_uploaded_bytes: number;
    total_downloaded_bytes: number;
    total_uploaded_mb: number;
    total_downloaded_mb: number;
    total_uploaded_gb: number;
    total_downloaded_gb: number;
    days_count: number;
}

interface NetworkLog {
    id: number;
    date: string;
    uploaded_bytes: number;
    downloaded_bytes: number;
    created_at: string;
    updated_at: string;
}

export default function NetworkUsage() {
    const [selectedPeriod, setSelectedPeriod] = useState<string>("Today");
    const [stats, setStats] = useState<NetworkStats | null>(null);
    const [logs, setLogs] = useState<NetworkLog[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [showLogs, setShowLogs] = useState(false);
    const [customStartDate, setCustomStartDate] = useState("");
    const [customEndDate, setCustomEndDate] = useState("");

    useEffect(() => {
        loadStats();
    }, [selectedPeriod, customStartDate, customEndDate]);

    const buildTimePeriod = (): TimePeriodType => {
        if (selectedPeriod === "Custom" && customStartDate && customEndDate) {
            return {
                type: "Custom",
                start_date: customStartDate,
                end_date: customEndDate,
            };
        }
        return { type: selectedPeriod as any };
    };

    const loadStats = async () => {
        if (selectedPeriod === "Custom" && (!customStartDate || !customEndDate)) {
            return;
        }

        setLoading(true);
        setError(null);

        try {
            const period = buildTimePeriod();
            const result = await invoke<NetworkStats>("get_network_stats", { period });
            setStats(result);
        } catch (err) {
            setError(`Failed to load stats: ${err}`);
            setStats(null);
        } finally {
            setLoading(false);
        }
    };

    const loadLogs = async () => {
        if (selectedPeriod === "Custom" && (!customStartDate || !customEndDate)) {
            return;
        }

        setLoading(true);
        setError(null);

        try {
            const period = buildTimePeriod();
            const result = await invoke<NetworkLog[]>("get_network_logs", { period });
            setLogs(result);
            setShowLogs(true);
        } catch (err) {
            setError(`Failed to load logs: ${err}`);
            setLogs([]);
        } finally {
            setLoading(false);
        }
    };

    const formatBytes = (bytes: number): string => {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
    };

    const formatDate = (dateStr: string): string => {
        const date = new Date(dateStr);
        return date.toLocaleDateString(undefined, {
            year: "numeric",
            month: "short",
            day: "numeric",
        });
    };

    return (
        <section className="network-usage-section">
            <h2>Network Usage</h2>

            <div className="period-selector">
                <label htmlFor="period">Time Period:</label>
                <select
                    id="period"
                    value={selectedPeriod}
                    onChange={(e) => {
                        setSelectedPeriod(e.target.value);
                        setShowLogs(false);
                    }}
                >
                    <option value="Today">Today</option>
                    <option value="Yesterday">Yesterday</option>
                    <option value="ThisWeek">This Week</option>
                    <option value="LastWeek">Last Week</option>
                    <option value="ThisMonth">This Month</option>
                    <option value="LastMonth">Last Month</option>
                    <option value="ThisYear">This Year</option>
                    <option value="LastYear">Last Year</option>
                    <option value="Custom">Custom Range</option>
                </select>
            </div>

            {selectedPeriod === "Custom" && (
                <div className="custom-date-range">
                    <div className="date-input">
                        <label htmlFor="start-date">Start Date:</label>
                        <input
                            id="start-date"
                            type="date"
                            value={customStartDate}
                            onChange={(e) => setCustomStartDate(e.target.value)}
                        />
                    </div>
                    <div className="date-input">
                        <label htmlFor="end-date">End Date:</label>
                        <input
                            id="end-date"
                            type="date"
                            value={customEndDate}
                            onChange={(e) => setCustomEndDate(e.target.value)}
                        />
                    </div>
                </div>
            )}

            {loading && <p className="loading">Loading...</p>}
            {error && <p className="error">{error}</p>}

            {stats && !loading && (
                <div className="stats-container">
                    <div className="stats-header">
                        <h3>Statistics</h3>
                        <p className="date-range">
                            {formatDate(stats.start_date)} - {formatDate(stats.end_date)}
                            {stats.days_count > 0 && ` (${stats.days_count} days)`}
                        </p>
                    </div>

                    <div className="stats-grid">
                        <div className="stat-card upload">
                            <div className="stat-icon">↑</div>
                            <div className="stat-content">
                                <h4>Uploaded</h4>
                                <p className="stat-value">{stats.total_uploaded_gb.toFixed(2)} GB</p>
                                <p className="stat-detail">{stats.total_uploaded_mb.toFixed(2)} MB</p>
                                <p className="stat-bytes">{formatBytes(stats.total_uploaded_bytes)}</p>
                            </div>
                        </div>

                        <div className="stat-card download">
                            <div className="stat-icon">↓</div>
                            <div className="stat-content">
                                <h4>Downloaded</h4>
                                <p className="stat-value">{stats.total_downloaded_gb.toFixed(2)} GB</p>
                                <p className="stat-detail">{stats.total_downloaded_mb.toFixed(2)} MB</p>
                                <p className="stat-bytes">{formatBytes(stats.total_downloaded_bytes)}</p>
                            </div>
                        </div>

                        <div className="stat-card total">
                            <div className="stat-icon">∑</div>
                            <div className="stat-content">
                                <h4>Total</h4>
                                <p className="stat-value">
                                    {(stats.total_uploaded_gb + stats.total_downloaded_gb).toFixed(2)} GB
                                </p>
                                <p className="stat-detail">
                                    {(stats.total_uploaded_mb + stats.total_downloaded_mb).toFixed(2)} MB
                                </p>
                                <p className="stat-bytes">
                                    {formatBytes(stats.total_uploaded_bytes + stats.total_downloaded_bytes)}
                                </p>
                            </div>
                        </div>
                    </div>

                    <div className="actions">
                        <button onClick={loadLogs} disabled={loading}>
                            {showLogs ? "Refresh Daily Logs" : "View Daily Logs"}
                        </button>
                    </div>
                </div>
            )}

            {showLogs && logs.length > 0 && (
                <div className="logs-container">
                    <h3>Daily Logs</h3>
                    <div className="logs-table-wrapper">
                        <table className="logs-table">
                            <thead>
                                <tr>
                                    <th>Date</th>
                                    <th>Uploaded</th>
                                    <th>Downloaded</th>
                                    <th>Total</th>
                                </tr>
                            </thead>
                            <tbody>
                                {logs.map((log) => (
                                    <tr key={log.id}>
                                        <td>{formatDate(log.date)}</td>
                                        <td className="upload-cell">{formatBytes(log.uploaded_bytes)}</td>
                                        <td className="download-cell">{formatBytes(log.downloaded_bytes)}</td>
                                        <td className="total-cell">
                                            {formatBytes(log.uploaded_bytes + log.downloaded_bytes)}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}

            {showLogs && logs.length === 0 && !loading && (
                <p className="no-data">No data available for this period.</p>
            )}
        </section>
    );
}
